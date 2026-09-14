use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;

use wow_shared::{ShardFile, WorldConfig};

use crate::catalog::Catalog;
use crate::creature::Creature;
use crate::directory::Directory;
use crate::gameobject::GameObject;
use crate::map_handle::MapHandle;
use crate::player::Player;
use crate::rpc::MapSession;
use crate::world::{Chat, ChatChannel, PlayerMailbox, World};

#[derive(Clone)]
pub struct JoinResult {
    pub others: Vec<Player>,
    pub creatures: Vec<Creature>,
    pub gameobjects: Vec<GameObject>,
}

#[derive(Clone)]
pub struct MapRouter {
    local: HashMap<u32, World>,
    remote: HashMap<u32, SocketAddr>,
    directory: Directory,
}

impl MapRouter {
    pub fn local_maps(
        maps: impl IntoIterator<Item = u32>,
        directory: Directory,
        catalog: Option<std::sync::Arc<Catalog>>,
    ) -> Self {
        let local = maps
            .into_iter()
            .map(|map_id| {
                let world = match catalog.clone() {
                    Some(catalog) => World::with_catalog(map_id, catalog),
                    None => World::for_map(map_id),
                };
                (map_id, world)
            })
            .collect();
        Self {
            local,
            remote: HashMap::new(),
            directory,
        }
    }

    pub fn from_shards(shards: &ShardFile, directory: Directory) -> Self {
        Self::local_maps(shards.maps(), directory, None)
    }

    pub fn from_config(
        config: &WorldConfig,
        directory: Directory,
        catalog: Option<std::sync::Arc<Catalog>>,
    ) -> Self {
        if config.map_endpoints.is_empty() {
            Self::local_maps(config.hosted_maps(), directory, catalog)
        } else {
            Self {
                local: HashMap::new(),
                remote: config.map_endpoints.clone(),
                directory,
            }
        }
    }

    pub fn directory(&self) -> &Directory {
        &self.directory
    }

    pub fn map(&self, map_id: u32) -> Option<&World> {
        self.local.get(&map_id)
    }

    pub fn worlds(&self) -> impl Iterator<Item = &World> {
        self.local.values()
    }

    pub fn endpoint(&self, map_id: u32) -> Option<SocketAddr> {
        self.remote.get(&map_id).copied()
    }

    pub fn is_remote(&self) -> bool {
        !self.remote.is_empty()
    }

    pub fn join(&self, map_id: u32, player: Player, mailbox: PlayerMailbox) -> Option<JoinResult> {
        let world = self.local.get(&map_id)?;
        self.directory
            .register(player.guid, player.name.clone(), map_id, mailbox.clone());
        let position = player.position;
        let others = MapHandle::join(world, player, mailbox);
        Some(JoinResult {
            others,
            creatures: world.creatures_near(position),
            gameobjects: world.gameobjects_near(position),
        })
    }

    pub fn leave(&self, map_id: u32, guid: u64, mailbox: &PlayerMailbox) {
        if let Some(world) = self.local.get(&map_id) {
            world.leave(guid, mailbox);
        }
        self.directory.unregister(guid, mailbox);
    }

    pub fn transfer(
        &self,
        from_map: u32,
        to_map: u32,
        player: Player,
        mailbox: PlayerMailbox,
    ) -> Option<JoinResult> {
        if let Some(world) = self.local.get(&from_map) {
            world.leave(player.guid, &mailbox);
        }
        self.join(to_map, player, mailbox)
    }

    pub fn speak(&self, map_id: u32, from: &PlayerMailbox, chat: Chat) {
        match &chat.channel {
            ChatChannel::Whisper { to } => {
                self.directory
                    .whisper(from, chat.speaker, to.clone(), chat.text);
            }
            _ => {
                if let Some(world) = self.local.get(&map_id) {
                    world.speak(from, chat);
                }
            }
        }
    }

    pub fn tick_all(&self, now: Instant) {
        for world in self.local.values() {
            world.tick(now);
        }
    }
}

/// Live connection to the map that currently owns a player.
pub enum MapBackend {
    Local(World),
    Remote(MapSession),
}

impl MapBackend {
    pub fn map_id(&self) -> u32 {
        match self {
            Self::Local(world) => MapHandle::map_id(world),
            Self::Remote(session) => session.map_id(),
        }
    }

    pub fn leave(&self, guid: u64, mailbox: &PlayerMailbox) {
        match self {
            Self::Local(world) => MapHandle::leave(world, guid, mailbox),
            Self::Remote(session) => session.leave(guid),
        }
    }
}

pub struct WorldPresence {
    directory: Directory,
    map_id: u32,
    guid: u64,
    mailbox: PlayerMailbox,
    backend: Option<MapBackend>,
}

impl WorldPresence {
    pub fn new(
        directory: Directory,
        guid: u64,
        mailbox: PlayerMailbox,
        backend: MapBackend,
    ) -> Self {
        let map_id = backend.map_id();
        Self {
            directory,
            map_id,
            guid,
            mailbox,
            backend: Some(backend),
        }
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub fn backend(&self) -> Option<&MapBackend> {
        self.backend.as_ref()
    }

    pub fn set_backend(&mut self, backend: MapBackend) {
        if let Some(old) = self.backend.take() {
            old.leave(self.guid, &self.mailbox);
        }
        self.map_id = backend.map_id();
        self.backend = Some(backend);
        self.directory.set_map(self.guid, self.map_id);
    }
}

impl Drop for WorldPresence {
    fn drop(&mut self) {
        if let Some(backend) = self.backend.take() {
            backend.leave(self.guid, &self.mailbox);
        }
        self.directory.unregister(self.guid, &self.mailbox);
    }
}

#[cfg(test)]
#[path = "../test/router.rs"]
mod tests;
