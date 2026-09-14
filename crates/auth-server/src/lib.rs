mod accounts;
mod http;
mod login;
mod store;

pub use store::SessionStore;

use std::net::SocketAddr;

use tokio::net::TcpListener;
use wow_shared::AuthConfig;

use crate::accounts::AccountStore;

pub async fn serve(config: AuthConfig) -> anyhow::Result<()> {
    let login_listener = TcpListener::bind(config.bind).await?;
    let http_listener = TcpListener::bind(config.internal_bind).await?;
    let pool = accounts::connect_auth(&config.database_url).await?;
    serve_with_store(
        login_listener,
        http_listener,
        config.world_public_addr,
        SessionStore::new(config.redis_url),
        AccountStore::postgres(pool),
        config.internal_token,
    )
    .await
}

pub async fn serve_with_listeners(
    login_listener: TcpListener,
    http_listener: TcpListener,
    world_public_addr: String,
) -> anyhow::Result<()> {
    serve_with_store(
        login_listener,
        http_listener,
        world_public_addr,
        SessionStore::memory(),
        AccountStore::memory_with_user1()?,
        None,
    )
    .await
}

async fn serve_with_store(
    login_listener: TcpListener,
    http_listener: TcpListener,
    world_public_addr: String,
    store: SessionStore,
    accounts: AccountStore,
    internal_token: Option<String>,
) -> anyhow::Result<()> {
    let login_addr = login_listener.local_addr()?;
    let http_addr = http_listener.local_addr()?;
    tracing::info!(%login_addr, %http_addr, world = %world_public_addr, "auth-server listening");

    let login = tokio::spawn(login::accept_loop(
        login_listener,
        store.clone(),
        accounts.clone(),
        world_public_addr,
    ));
    let http = tokio::spawn(http::serve(http_listener, store, accounts, internal_token));
    tracing::info!(%login_addr, %http_addr, "auth-server ready");

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
