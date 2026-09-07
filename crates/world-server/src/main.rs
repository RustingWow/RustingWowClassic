use world_server::serve;
use wow_shared::{WorldConfig, init_tracing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    serve(WorldConfig::from_env()?).await
}
