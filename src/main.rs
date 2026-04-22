use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

use math_calc::server::MathCalcServer;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("math-calc-mcp starting on stdio transport");

    let service = MathCalcServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!(error = ?e, "failed to start stdio service"))?;

    service.waiting().await?;
    Ok(())
}
