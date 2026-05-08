use uof_agent::{AgentApplication, AgentConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = AgentApplication::new(AgentConfig::default())?;
    app.run().await
}
