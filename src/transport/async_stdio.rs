//! Non-blocking stdio transport for the MCP server.
//!
//! `rmcp::transport::stdio` hands back `tokio::io::Stdin`/`Stdout`, which
//! dispatch every read/write through a worker thread because `std::io::Stdin`
//! is synchronous. On a pipe-backed subprocess (the canonical MCP deployment)
//! that trip is pure overhead: the fd is already pollable by epoll (Linux),
//! kqueue (macOS / *BSD), or IOCP (Windows).
//!
//! This module wraps raw fds 0 and 1 in `tokio::io::unix::AsyncFd`, flips
//! them into `O_NONBLOCK`, and implements `AsyncRead`/`AsyncWrite` directly
//! against the selector. Two worker threads disappear from the runtime and
//! per-call latency drops by the thread-hop cost (tens of microseconds on
//! typical hardware).
//!
//! # Safety invariants
//! * **Never close fd 0 or fd 1.** Closing them mid-process would sever the
//!   parent's stdin/stdout pipe. The `StdinRaw`/`StdoutRaw` wrappers own
//!   nothing and explicitly do not implement `Drop`-style teardown.
//! * `O_NONBLOCK` is set on construction and left alone. Other threads
//!   touching the same fd observe the non-blocking flag.

#[cfg(unix)]
pub use unix::async_stdio;

#[cfg(not(unix))]
pub use fallback::async_stdio;

// -------------------------------------------------------------------------- //
//  Unix implementation — AsyncFd over raw fds 0/1
// -------------------------------------------------------------------------- //
#[cfg(unix)]
mod unix {
    use std::io;
    use std::os::fd::{AsRawFd, RawFd};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use tokio::io::unix::AsyncFd;
    use tokio::io::{AsyncRead, AsyncWrite, Interest, ReadBuf};

    /// Type-erased pair the rest of the binary treats as a `(R, W)` tuple for
    /// `rmcp::ServiceExt::serve`.
    ///
    /// # Panics
    /// Panics if `fcntl(F_GETFL)` / `fcntl(F_SETFL, O_NONBLOCK)` fail on
    /// stdin (fd 0) or stdout (fd 1), or if tokio cannot register them with
    /// the reactor. Both cases indicate a broken process environment (the
    /// parent closed our standard streams before we ran) — there is no
    /// recoverable path from here and aborting early is the honest answer.
    #[must_use]
    pub fn async_stdio() -> (AsyncStdin, AsyncStdout) {
        let stdin = AsyncStdin::new().expect("fd 0 must be ready before start");
        let stdout = AsyncStdout::new().expect("fd 1 must be ready before start");
        (stdin, stdout)
    }

    /// Adjust the file status flags to include `O_NONBLOCK`. We read the
    /// current flag set (fcntl `F_GETFL`) and OR in the bit, so pre-existing
    /// flags (e.g. `O_APPEND` on stdout) are preserved.
    fn set_non_blocking(fd: RawFd) -> io::Result<()> {
        // SAFETY: `fcntl` with `F_GETFL` takes no extra argument; it simply
        // returns the current flags or -1 on error.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_NONBLOCK != 0 {
            return Ok(());
        }
        // SAFETY: `F_SETFL` takes an int-sized flag word. Combining the
        // existing value with `O_NONBLOCK` cannot narrow to an invalid state.
        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Thin wrapper around a borrowed fd. Implements only `AsRawFd`, never
    /// `Drop`, so the `AsyncFd` layer cannot accidentally close the standard
    /// stream the parent gave us.
    ///
    /// `#[repr(transparent)]` keeps the ABI a plain `RawFd` in case the
    /// wrapper leaks through `Debug` or similar.
    #[repr(transparent)]
    struct BorrowedFd(RawFd);

    impl AsRawFd for BorrowedFd {
        fn as_raw_fd(&self) -> RawFd {
            self.0
        }
    }

    // -------- stdin -------- //

    /// Non-blocking stdin backed by `AsyncFd` on raw fd 0.
    pub struct AsyncStdin {
        inner: AsyncFd<BorrowedFd>,
    }

    impl AsyncStdin {
        fn new() -> io::Result<Self> {
            const STDIN_FD: RawFd = 0;
            set_non_blocking(STDIN_FD)?;
            let inner = AsyncFd::with_interest(BorrowedFd(STDIN_FD), Interest::READABLE)?;
            Ok(Self { inner })
        }
    }

    impl AsyncRead for AsyncStdin {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            loop {
                let mut guard = std::task::ready!(self.inner.poll_read_ready(cx))?;
                // Fill the uninitialised tail of the ReadBuf. `unfilled_mut`
                // returns `&mut [MaybeUninit<u8>]`; `libc::read` writes raw
                // bytes so we cast after a zero-length sanity check.
                let unfilled = buf.initialize_unfilled();
                if unfilled.is_empty() {
                    return Poll::Ready(Ok(()));
                }
                let read_result = guard.try_io(|fd| {
                    // SAFETY: `read` takes (fd, buf, len) and writes `len`
                    // bytes at most into `buf`. `unfilled` is a live `&mut
                    // [u8]` borrow so the pointer and length are valid.
                    let n = unsafe {
                        libc::read(fd.as_raw_fd(), unfilled.as_mut_ptr().cast(), unfilled.len())
                    };
                    if n < 0 {
                        Err(io::Error::last_os_error())
                    } else {
                        // `read` cannot return negative values past this
                        // point and cannot exceed `unfilled.len()`, which
                        // already fits `usize`. Branch is defensive rather
                        // than a silent cast.
                        usize::try_from(n)
                            .map_err(|_| io::Error::other("read returned out-of-range byte count"))
                    }
                });
                match read_result {
                    Ok(Ok(n)) => {
                        buf.advance(n);
                        return Poll::Ready(Ok(()));
                    }
                    Ok(Err(e)) => return Poll::Ready(Err(e)),
                    // try_io already cleared readiness; the outer loop
                    // re-arms the waker via poll_read_ready.
                    Err(_would_block) => {}
                }
            }
        }
    }

    // -------- stdout -------- //

    /// Non-blocking stdout backed by `AsyncFd` on raw fd 1.
    pub struct AsyncStdout {
        inner: AsyncFd<BorrowedFd>,
    }

    impl AsyncStdout {
        fn new() -> io::Result<Self> {
            const STDOUT_FD: RawFd = 1;
            set_non_blocking(STDOUT_FD)?;
            let inner = AsyncFd::with_interest(BorrowedFd(STDOUT_FD), Interest::WRITABLE)?;
            Ok(Self { inner })
        }
    }

    impl AsyncWrite for AsyncStdout {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            loop {
                let mut guard = std::task::ready!(self.inner.poll_write_ready(cx))?;
                let write_result = guard.try_io(|fd| {
                    // SAFETY: `write` reads `buf.len()` bytes starting at
                    // `buf.as_ptr()`. The slice is alive for the duration of
                    // the call.
                    let n = unsafe { libc::write(fd.as_raw_fd(), buf.as_ptr().cast(), buf.len()) };
                    if n < 0 {
                        Err(io::Error::last_os_error())
                    } else {
                        usize::try_from(n)
                            .map_err(|_| io::Error::other("write returned out-of-range byte count"))
                    }
                });
                match write_result {
                    Ok(result) => return Poll::Ready(result),
                    Err(_would_block) => {}
                }
            }
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            // Pipes and TTYs have no userspace buffer we own; kernel drains
            // on its own schedule, so flush is effectively a no-op here.
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            // Closing fd 1 would terminate the parent's pipe; shutdown is a
            // no-op. The OS reclaims the fd at process exit.
            Poll::Ready(Ok(()))
        }
    }

    // ---------------- tests ---------------- //

    #[cfg(test)]
    mod tests {
        use super::*;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        /// Build a pipe and return `(AsyncStdin-like reader, AsyncStdout-like
        /// writer)` both in non-blocking mode and tokio-aware. Matches the
        /// exact codepath `async_stdio()` takes for fd 0 / 1 except the fd
        /// numbers come from `pipe2` instead of being hard-coded.
        fn pollable_pipe() -> (AsyncStdin, AsyncStdout) {
            let mut fds: [RawFd; 2] = [0; 2];
            // `pipe2` is Linux-only; portable code uses `pipe` + `fcntl`.
            // SAFETY: `pipe` writes two valid fds into `fds` or returns -1.
            let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
            assert_eq!(rc, 0, "pipe failed: {}", io::Error::last_os_error());
            set_non_blocking(fds[0]).unwrap();
            set_non_blocking(fds[1]).unwrap();
            // The test synthesises the same wiring as the real constructors
            // but against the pipe's read/write ends.
            let reader = AsyncStdin {
                inner: AsyncFd::with_interest(BorrowedFd(fds[0]), Interest::READABLE).unwrap(),
            };
            let writer = AsyncStdout {
                inner: AsyncFd::with_interest(BorrowedFd(fds[1]), Interest::WRITABLE).unwrap(),
            };
            (reader, writer)
        }

        /// The pipe test leaks fds intentionally — our `BorrowedFd` must not
        /// close them, and each test gets a fresh pair. The kernel cleans up
        /// at process exit.
        #[allow(dead_code)]
        const _FD_LEAK_ACKNOWLEDGED: () = ();

        #[tokio::test]
        async fn roundtrip_short_message() {
            let (mut r, mut w) = pollable_pipe();
            let payload = b"ping-async\n";
            w.write_all(payload).await.unwrap();
            let mut buf = vec![0u8; payload.len()];
            r.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, payload);
        }

        #[tokio::test]
        async fn reader_waits_for_writer_across_yield() {
            // Exercises the epoll/kqueue wake-up path: the reader task parks
            // on `poll_read_ready`, the writer runs on another timer slot,
            // and the selector resumes the reader when data arrives.
            let (mut r, mut w) = pollable_pipe();
            let writer_task = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                w.write_all(b"delayed").await.unwrap();
                w.shutdown().await.unwrap();
            });
            let mut buf = [0u8; 7];
            r.read_exact(&mut buf).await.unwrap();
            writer_task.await.unwrap();
            assert_eq!(&buf, b"delayed");
        }

        #[tokio::test]
        async fn set_non_blocking_is_idempotent() {
            // Re-applying the flag to an already non-blocking fd is fine —
            // the short-circuit in `set_non_blocking` avoids the extra
            // fcntl syscall.
            let mut fds: [RawFd; 2] = [0; 2];
            unsafe {
                libc::pipe(fds.as_mut_ptr());
            }
            set_non_blocking(fds[0]).unwrap();
            set_non_blocking(fds[0]).unwrap();
            // Clean up both ends so the test doesn't leak.
            unsafe {
                libc::close(fds[0]);
                libc::close(fds[1]);
            }
        }
    }
}

// -------------------------------------------------------------------------- //
//  Windows fallback — IOCP via `tokio::io::stdin/stdout`
// -------------------------------------------------------------------------- //
#[cfg(not(unix))]
mod fallback {
    use tokio::io::{Stdin, Stdout};

    #[must_use]
    pub fn async_stdio() -> (Stdin, Stdout) {
        (tokio::io::stdin(), tokio::io::stdout())
    }
}
