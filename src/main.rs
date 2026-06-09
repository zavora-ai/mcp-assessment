use mcp_assessment::server::AssessmentServer;
use mcp_assessment::store::AssessmentStore;
use rmcp::{ServiceExt, transport::stdio};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()),
        )
        .init();
    let store = Arc::new(AssessmentStore::new());
    let server = AssessmentServer { store };
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
