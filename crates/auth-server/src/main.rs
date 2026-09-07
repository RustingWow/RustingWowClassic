use auth_server::serve;
use wow_shared::{AuthConfig, init_tracing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    serve(AuthConfig::from_env()?).await
}
