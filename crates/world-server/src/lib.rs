mod creature;
mod player;
mod protocol;
mod session;
mod world;

use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use wow_shared::WorldConfig;

use crate::world::World;

pub async fn serve(config: WorldConfig) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.bind).await?;
    serve_with_listener(
        listener,
        config.auth_internal_url,
        config.log_unhandled_packets,
    )
    .await
}

pub async fn serve_with_listener(
    listener: TcpListener,
    auth_internal_url: String,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let addr = listener.local_addr()?;
    tracing::info!(
        %addr,
        auth = %auth_internal_url,
        log_unhandled_packets,
        "world-server listening"
    );

    let world = World::new();
    let ticker = world.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        loop {
            interval.tick().await;
            ticker.tick(Instant::now());
        }
    });
    loop {
        let (stream, peer) = listener.accept().await?;
        let auth_internal_url = auth_internal_url.clone();
        let world = world.clone();
        tokio::spawn(async move {
            if let Err(error) =
                session::handle_client(stream, auth_internal_url, log_unhandled_packets, world)
                    .await
            {
                tracing::warn!(%peer, %error, "world session ended");
            }
        });
    }
}
