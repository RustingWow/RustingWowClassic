mod enter_world;
mod packet;
mod session;

use tokio::net::TcpListener;
use wow_shared::WorldConfig;

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

    loop {
        let (stream, peer) = listener.accept().await?;
        let auth_internal_url = auth_internal_url.clone();
        tokio::spawn(async move {
            if let Err(error) =
                session::handle_client(stream, auth_internal_url, log_unhandled_packets).await
            {
                tracing::warn!(%peer, %error, "world session ended");
            }
        });
    }
}
