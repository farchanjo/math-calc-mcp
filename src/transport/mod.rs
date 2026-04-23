//! Transport layers used by the MCP stdio binary.
//!
//! [`async_stdio`] is the fully non-blocking replacement for
//! `rmcp::transport::stdio`. On Unix it wires file descriptors 0 and 1
//! through `tokio::io::unix::AsyncFd`, which dispatches through the
//! platform selector (epoll on Linux, kqueue on macOS / *BSD). No worker
//! threads, no blocking reads, no copies beyond the single kernel buffer.
//!
//! On Windows we fall back to the stock `tokio::io::stdin/stdout`, which
//! already rides IOCP for pipe handles; `AsyncFd` is not available there.

pub mod async_stdio;

pub use async_stdio::async_stdio;
