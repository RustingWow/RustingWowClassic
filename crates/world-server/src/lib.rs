mod appearance;
mod catalog;
mod character_store;
mod creature;
mod db;
mod directory;
mod map_handle;
mod player;
mod protocol;
mod quest_director;
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
use crate::catalog::Catalog;
use crate::directory::Directory;
use crate::map_handle::MapHandle;
use crate::router::MapRouter;
use crate::world::World;

pub async fn serve(config: WorldConfig) -> anyhow::Result<()> {
    match config.role {
        WorldRole::Map => {
            let pool = db::connect_world(&config.database_url).await?;
            let catalog = load_catalog(&pool).await;
            serve_map_role(config, catalog).await
        }
        WorldRole::Gateway | WorldRole::Combined => {
            let pool = db::connect_world(&config.database_url).await?;
            let catalog = load_catalog(&pool).await;
            let characters = CharacterStore::postgres(pool).await?;
            let listener = TcpListener::bind(config.bind).await?;
            serve_gateway_role(listener, config, characters, catalog).await
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
    serve_gateway_role(listener, config, CharacterStore::memory(), None).await
}

async fn load_catalog(pool: &sqlx::PgPool) -> Option<std::sync::Arc<Catalog>> {
    match Catalog::load(pool).await {
        Ok(catalog) if !catalog.is_empty() => Some(catalog),
        Ok(_) => {
            tracing::warn!("world catalog is empty; apply db/load.sh after migrate");
            None
        }
        Err(error) => {
            tracing::warn!(%error, "world catalog not loaded");
            None
        }
    }
}

async fn serve_gateway_role(
    listener: TcpListener,
    config: WorldConfig,
    characters: CharacterStore,
    catalog: Option<std::sync::Arc<Catalog>>,
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
    let router = MapRouter::from_config(&config, directory, catalog);
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

async fn serve_map_role(
    config: WorldConfig,
    catalog: Option<std::sync::Arc<Catalog>>,
) -> anyhow::Result<()> {
    let maps: HashMap<_, _> = config
        .hosted_maps()
        .into_iter()
        .map(|map_id| {
            let world = match catalog.clone() {
                Some(catalog) => World::with_catalog(map_id, catalog),
                None => World::for_map(map_id),
            };
            (map_id, world)
        })
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
