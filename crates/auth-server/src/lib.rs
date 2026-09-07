mod http;
mod login;
mod store;

pub use store::SessionStore;

use std::net::SocketAddr;

use tokio::net::TcpListener;
use wow_shared::AuthConfig;

pub async fn serve(config: AuthConfig) -> anyhow::Result<()> {
    let login_listener = TcpListener::bind(config.bind).await?;
    let http_listener = TcpListener::bind(config.internal_bind).await?;
    serve_with_listeners(login_listener, http_listener, config.world_public_addr).await
}

pub async fn serve_with_listeners(
    login_listener: TcpListener,
    http_listener: TcpListener,
    world_public_addr: String,
) -> anyhow::Result<()> {
    let store = SessionStore::new();
    let login_addr = login_listener.local_addr()?;
    let http_addr = http_listener.local_addr()?;
    tracing::info!(%login_addr, %http_addr, world = %world_public_addr, "auth-server listening");

    let login = tokio::spawn(login::accept_loop(
        login_listener,
        store.clone(),
        world_public_addr,
    ));
    let http = tokio::spawn(http::serve(http_listener, store));

    tokio::select! {
        result = login => result??,
        result = http => result??,
    }
    Ok(())
}

pub fn bound_addrs(
    login: &TcpListener,
    http: &TcpListener,
) -> anyhow::Result<(SocketAddr, SocketAddr)> {
    Ok((login.local_addr()?, http.local_addr()?))
}
