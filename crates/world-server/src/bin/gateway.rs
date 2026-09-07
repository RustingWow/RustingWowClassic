use world_server::serve;
use wow_shared::{WorldConfig, WorldRole, init_tracing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let mut config = WorldConfig::from_env()?;
    config.role = WorldRole::Gateway;
    serve(config).await
}
