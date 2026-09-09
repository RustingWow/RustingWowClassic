mod appearance;
mod character_store;
mod creature;
mod db;
mod directory;
mod map_handle;
mod player;
mod protocol;
mod race_starts;
mod router;
mod rpc;
mod session;
mod world;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use wow_shared::{ShardFile, WorldConfig, WorldRole};

use crate::character_store::CharacterStore;
use crate::directory::Directory;
use crate::map_handle::MapHandle;
use crate::router::MapRouter;
use crate::world::World;

pub async fn serve(config: WorldConfig) -> anyhow::Result<()> {
    match config.role {
        WorldRole::Map => {
            let _pool = db::connect_world(&config.database_url).await?;
            serve_map_role(config).await
        }
        WorldRole::Gateway | WorldRole::Combined => {
            let pool = db::connect_world(&config.database_url).await?;
            let listener = TcpListener::bind(config.bind).await?;
            serve_gateway_role(listener, config, CharacterStore::postgres(pool)).await
        }
    }
}

pub async fn serve_with_listener(
    listener: TcpListener,
    auth_internal_url: String,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let config = WorldConfig {
        bind: listener.local_addr()?,
        auth_internal_url,
        log_unhandled_packets,
        shards: ShardFile::builtin(),
        role: WorldRole::Combined,
        map_ids: Vec::new(),
        map_bind: "127.0.0.1:0".parse()?,
        map_endpoints: HashMap::new(),
        redis_url: None,
        database_url: String::new(),
    };
    serve_gateway_role(listener, config, CharacterStore::memory()).await
}

async fn serve_gateway_role(
    listener: TcpListener,
    config: WorldConfig,
    characters: CharacterStore,
) -> anyhow::Result<()> {
    let addr = listener.local_addr()?;
    tracing::info!(
        %addr,
        auth = %config.auth_internal_url,
        role = ?config.role,
        log_unhandled_packets = config.log_unhandled_packets,
        "world session listening"
    );

    let directory = directory_from_config(&config);
    let router = MapRouter::from_config(&config, directory);
    spawn_local_tickers(&router);

    loop {
        let (stream, peer) = listener.accept().await?;
        let auth_internal_url = config.auth_internal_url.clone();
        let log_unhandled_packets = config.log_unhandled_packets;
        let router = router.clone();
        let characters = characters.clone();
        tokio::spawn(async move {
            if let Err(error) = session::handle_client(
                stream,
                auth_internal_url,
                log_unhandled_packets,
                router,
                characters,
            )
            .await
            {
                tracing::warn!(%peer, %error, "world session ended");
            }
        });
    }
}

async fn serve_map_role(config: WorldConfig) -> anyhow::Result<()> {
    let maps: HashMap<_, _> = config
        .hosted_maps()
        .into_iter()
        .map(|map_id| (map_id, World::for_map(map_id)))
        .collect();
    rpc::spawn_map_tickers(&maps);
    rpc::serve_maps(config.map_bind, maps).await
}

fn directory_from_config(config: &WorldConfig) -> Directory {
    match &config.redis_url {
        Some(url) => Directory::with_redis(url.clone()),
        None => Directory::memory(),
    }
}

fn spawn_local_tickers(router: &MapRouter) {
    for world in router.worlds() {
        let ticker = world.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(200));
            loop {
                interval.tick().await;
                MapHandle::tick(&ticker, Instant::now());
            }
        });
    }
}
