use anyhow::Result;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

use math_calc::server::MathCalcServer;
use math_calc::transport::async_stdio;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("math-calc-mcp starting on non-blocking stdio transport (AsyncFd over fd 0/1)");

    // `async_stdio()` flips fd 0/1 to `O_NONBLOCK` and registers them with
    // tokio's mio-backed selector (epoll on Linux, kqueue on macOS/*BSD,
    // IOCP on Windows). No worker-thread hops per read/write.
    let service = MathCalcServer::new()
        .serve(async_stdio())
        .await
        .inspect_err(|e| tracing::error!(error = ?e, "failed to start stdio service"))?;

    service.waiting().await?;
    Ok(())
}
