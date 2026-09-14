mod chat;
mod combat;
mod gossip;
mod quest;
mod vendor;
mod visibility;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use wow_shared::{MAP_EASTERN_KINGDOMS, Position};
use wow_world_messages::vanilla::opcodes::ServerOpcodeMessage;

use crate::catalog::{Catalog, ItemRow, QuestRow};
use crate::creature::{Creature, northshire_npcs};
use crate::gameobject::GameObject;
use crate::player::Player;
use crate::quest_director::{QuestDirector, TaskSpec};

pub use chat::{Chat, ChatChannel, ChatDelivery, SpokenChat};
pub use combat::{Attack, MeleeHit};
pub use vendor::{LootOffer, VendorOffer};

pub(crate) const SAY_RANGE: f32 = 25.0;
pub(crate) const YELL_RANGE: f32 = 300.0;
pub(crate) const EMOTE_RANGE: f32 = 25.0;
pub(crate) const MELEE_RANGE: f32 = 5.0;
pub(crate) const AGGRO_RANGE: f32 = 18.0;
pub(crate) const LEASH_RANGE: f32 = 50.0;
pub(crate) const GOSSIP_RANGE: f32 = 10.0;
pub(crate) const SWING_INTERVAL: Duration = Duration::from_millis(2000);
pub(crate) const REGEN_INTERVAL: Duration = Duration::from_secs(2);
pub(crate) const COMBAT_REGEN_DELAY: Duration = Duration::from_secs(5);
pub(crate) const CELL_SIZE: f32 = 40.0;
pub(crate) const CREATE_RANGE: f32 = 90.0;
pub(crate) const DESTROY_RANGE: f32 = 100.0;
pub(crate) const CELL_HYSTERESIS: f32 = 10.0;
pub(crate) const CREATURE_SPEED: f32 = 7.0;
pub(crate) const TICK_SECS: f32 = 0.2;

/// Movement already translated for nearby clients; the packet body stays private.
#[derive(Clone, Debug)]
pub struct Movement {
    pub guid: u64,
    pub position: Position,
    packet: Option<ServerOpcodeMessage>,
}

impl Movement {
    pub(crate) fn new(guid: u64, position: Position, packet: ServerOpcodeMessage) -> Self {
        Self {
            guid,
            position,
            packet: Some(packet),
        }
    }

    pub fn without_packet(guid: u64, position: Position) -> Self {
        Self {
            guid,
            position,
            packet: None,
        }
    }

    pub(crate) fn packet(&self) -> Option<&ServerOpcodeMessage> {
        self.packet.as_ref()
    }
}

#[derive(Clone, Debug)]
pub enum WorldEvent {
    PlayerAppeared(Player),
    PlayerLeft {
        guid: u64,
    },
    PlayerMoved(Movement),
    Chat(SpokenChat),
    ChatPlayerNotFound {
        name: String,
    },
    CreatureAppeared(Creature),
    CreatureLeft {
        guid: u64,
    },
    GameObjectAppeared(GameObject),
    GameObjectLeft {
        guid: u64,
    },
    AttackStarted(Attack),
    AttackStopped(Attack),
    MeleeHit(MeleeHit),
    StandStateAck {
        state: u8,
    },
    PlayerStandState {
        guid: u64,
        state: u8,
    },
    GossipOpened {
        npc: u64,
        menu: crate::creature::GossipMenu,
        quests: Vec<crate::quest::GossipQuestItem>,
        pages: [crate::catalog::NpcTextPage; 8],
    },
    GossipClosed,
    QuestGiverStatus {
        npc: u64,
        status: u32,
    },
    QuestList {
        npc: u64,
        title: String,
        quests: Vec<crate::quest::GossipQuestItem>,
    },
    QuestDetails {
        npc: u64,
        quest: QuestRow,
    },
    QuestLogFull,
    QuestLogUpdate {
        player: Player,
    },
    QuestOfferReward {
        npc: u64,
        quest: QuestRow,
    },
    QuestTurnedIn {
        quest_id: u32,
        copper: u32,
        items: Vec<(u32, u32)>,
    },
    QuestKillCredit {
        victim: u64,
        credit: crate::quest::KillCredit,
    },
    QuestObjectivesDone {
        quest_id: u32,
    },
    QuestStateChanged {
        guid: u64,
        change: crate::quest::QuestStateChange,
    },
    VendorOpened {
        npc: u64,
        items: Vec<VendorOffer>,
    },
    LootOpened {
        guid: u64,
        gold: u32,
        items: Vec<LootOffer>,
    },
    LootFailed {
        guid: u64,
        error: u8,
    },
    LootTaken {
        index: u8,
    },
    LootClosed {
        guid: u64,
    },
    MoneyChanged {
        guid: u64,
        copper: u32,
    },
    Notification {
        text: String,
    },
    ForcedTeleport {
        map_id: u32,
        position: Position,
    },
    CreatureMoved {
        guid: u64,
        from: Position,
        to: Position,
        duration_ms: u32,
    },
}

#[derive(Clone)]
pub struct PlayerMailbox {
    tx: mpsc::UnboundedSender<WorldEvent>,
}

impl PlayerMailbox {
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<WorldEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    pub(crate) fn send(&self, event: WorldEvent) {
        let _ = self.tx.send(event);
    }

    pub(crate) fn same_channel(&self, other: &Self) -> bool {
        self.tx.same_channel(&other.tx)
    }
}

pub(crate) struct Presence {
    player: Player,
    mailbox: PlayerMailbox,
    attacking: Option<u64>,
    next_swing: Instant,
    next_regen: Instant,
    last_combat: Option<Instant>,
    visible: HashSet<u64>,
    last_cell: (i32, i32),
    looting: Option<u64>,
}

pub(crate) struct CreatureState {
    creature: Creature,
    home: Position,
    combat_target: Option<u64>,
    next_swing: Instant,
    died_at: Option<Instant>,
    damage: i32,
}

pub(crate) struct Inner {
    players: HashMap<u64, Presence>,
    creatures: HashMap<u64, CreatureState>,
    gameobjects: HashMap<u64, GameObject>,
    grid: HashMap<(i32, i32), HashSet<u64>>,
    loot: HashMap<u64, Vec<(u32, u32)>>,
    temp_counter: u32,
}

/// One map's player list, creatures, and combat tick.
#[derive(Clone)]
pub struct World {
    pub(crate) map_id: u32,
    pub(crate) inner: Arc<Mutex<Inner>>,
    pub(crate) catalog: Option<Arc<Catalog>>,
}

impl World {
    pub fn new() -> Self {
        Self::for_map(MAP_EASTERN_KINGDOMS)
    }

    pub fn for_map(map_id: u32) -> Self {
        Self::with_entities(map_id, creatures_for_map(map_id), Vec::new(), None)
    }

    pub fn with_catalog(map_id: u32, catalog: Arc<Catalog>) -> Self {
        let creatures = catalog.creatures_for_map(map_id);
        let creatures = if creatures.is_empty() {
            creatures_for_map(map_id)
        } else {
            creatures
        };
        let gameobjects = catalog.gameobjects_for_map(map_id);
        Self::with_entities(map_id, creatures, gameobjects, Some(catalog))
    }

    #[cfg(test)]
    pub fn with_creatures(creatures: Vec<Creature>, catalog: Option<Arc<Catalog>>) -> Self {
        Self::with_entities(MAP_EASTERN_KINGDOMS, creatures, Vec::new(), catalog)
    }

    #[cfg(test)]
    pub fn with_gameobjects(gameobjects: Vec<GameObject>) -> Self {
        Self::with_entities(
            MAP_EASTERN_KINGDOMS,
            creatures_for_map(MAP_EASTERN_KINGDOMS),
            gameobjects,
            None,
        )
    }

    fn with_entities(
        map_id: u32,
        creatures: Vec<Creature>,
        gameobjects: Vec<GameObject>,
        catalog: Option<Arc<Catalog>>,
    ) -> Self {
        let now = Instant::now();
        let mut grid: HashMap<(i32, i32), HashSet<u64>> = HashMap::new();
        let creatures = creatures
            .into_iter()
            .map(|creature| {
                let damage = if creature.melee_damage > 0 {
                    creature.melee_damage
                } else if creature.hostile {
                    4
                } else {
                    0
                };
                grid.entry(creature.position.cell(CELL_SIZE))
                    .or_default()
                    .insert(creature.guid);
                (
                    creature.guid,
                    CreatureState {
                        home: creature.position,
                        creature,
                        combat_target: None,
                        next_swing: now,
                        died_at: None,
                        damage,
                    },
                )
            })
            .collect();
        let gameobjects = gameobjects
            .into_iter()
            .map(|object| {
                grid.entry(object.position.cell(CELL_SIZE))
                    .or_default()
                    .insert(object.guid);
                (object.guid, object)
            })
            .collect();
        Self {
            map_id,
            catalog,
            inner: Arc::new(Mutex::new(Inner {
                players: HashMap::new(),
                creatures,
                gameobjects,
                grid,
                loot: HashMap::new(),
                temp_counter: 0xF000_0000,
            })),
        }
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub fn join(&self, player: Player, mailbox: PlayerMailbox) -> Vec<Player> {
        tracing::debug!(
            map_id = self.map_id,
            guid = player.guid,
            "player joined map"
        );
        let mut inner = self.inner.lock().expect("world mutex");
        if inner.players.remove(&player.guid).is_some() {
            broadcast(&inner, WorldEvent::PlayerLeft { guid: player.guid });
        }

        let others: Vec<_> = inner
            .players
            .values()
            .filter(|presence| presence.player.gm_visible)
            .map(|presence| presence.player.clone())
            .collect();
        if player.gm_visible {
            broadcast(&inner, WorldEvent::PlayerAppeared(player.clone()));
        }
        let nearby = visibility::creatures_in_range(&inner, player.position, CREATE_RANGE);
        let nearby_gos = visibility::gameobjects_in_range(&inner, player.position, CREATE_RANGE);
        let visible: HashSet<u64> = nearby
            .iter()
            .map(|creature| creature.guid)
            .chain(nearby_gos.iter().map(|object| object.guid))
            .collect();
        let last_cell = player.position.cell(CELL_SIZE);
        inner.players.insert(
            player.guid,
            Presence {
                player,
                mailbox,
                attacking: None,
                next_swing: Instant::now(),
                next_regen: Instant::now() + REGEN_INTERVAL,
                last_combat: None,
                visible,
                last_cell,
                looting: None,
            },
        );
        others
    }

    pub fn leave(&self, guid: u64, mailbox: &PlayerMailbox) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&guid) else {
            return;
        };
        if !presence.mailbox.same_channel(mailbox) {
            return;
        }
        combat::drop_combat_with(&mut inner, guid);
        inner.players.remove(&guid);
        broadcast(&inner, WorldEvent::PlayerLeft { guid });
    }

    pub fn player(&self, guid: u64) -> Option<Player> {
        self.inner
            .lock()
            .expect("world mutex")
            .players
            .get(&guid)
            .map(|presence| presence.player.clone())
    }

    pub fn creatures(&self) -> Vec<Creature> {
        self.inner
            .lock()
            .expect("world mutex")
            .creatures
            .values()
            .map(|state| state.creature.clone())
            .collect()
    }

    pub fn creature(&self, guid: u64) -> Option<Creature> {
        self.inner
            .lock()
            .expect("world mutex")
            .creatures
            .get(&guid)
            .map(|state| state.creature.clone())
    }

    pub fn creature_by_entry(&self, entry: u32) -> Option<Creature> {
        self.inner
            .lock()
            .expect("world mutex")
            .creatures
            .values()
            .find(|state| state.creature.entry == entry)
            .map(|state| state.creature.clone())
    }

    pub fn gameobject(&self, guid: u64) -> Option<GameObject> {
        self.inner
            .lock()
            .expect("world mutex")
            .gameobjects
            .get(&guid)
            .cloned()
    }

    pub fn gameobject_by_entry(&self, entry: u32) -> Option<GameObject> {
        self.inner
            .lock()
            .expect("world mutex")
            .gameobjects
            .values()
            .find(|object| object.entry == entry)
            .cloned()
    }

    pub fn change_stand_state(&self, from: &PlayerMailbox, guid: u64, state: u8) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&guid) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        if !presence.player.is_alive() {
            return;
        }
        presence.player.stand_state = state;
        from.send(WorldEvent::StandStateAck { state });
        broadcast(&inner, WorldEvent::PlayerStandState { guid, state });
    }

    pub fn npc_text(&self, text_id: u32) -> Option<String> {
        if let Some(text) = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.gossip_text(text_id))
        {
            return Some(text);
        }
        self.inner
            .lock()
            .expect("world mutex")
            .creatures
            .values()
            .find_map(|state| {
                state
                    .creature
                    .gossip
                    .as_ref()
                    .and_then(|gossip| gossip.menu(text_id))
                    .map(|menu| menu.text.clone())
            })
    }

    pub fn npc_text_pages(&self, text_id: u32) -> [crate::catalog::NpcTextPage; 8] {
        if let Some(catalog) = self.catalog.as_ref() {
            let pages = catalog.npc_text_pages(text_id);
            if pages
                .iter()
                .any(|page| !page.text.is_empty() || page.probability > 0.0)
            {
                return pages;
            }
        }
        let fallback = self.npc_text(text_id).unwrap_or_default();
        gossip::pages_for(self.catalog.as_deref(), text_id, &fallback)
    }

    pub fn quest(&self, entry: u32) -> Option<QuestRow> {
        self.catalog
            .as_ref()
            .and_then(|catalog| catalog.quest(entry).cloned())
    }

    pub fn item(&self, entry: u32) -> Option<ItemRow> {
        self.catalog
            .as_ref()
            .and_then(|catalog| catalog.item(entry).cloned())
    }

    pub fn catalog(&self) -> Option<std::sync::Arc<Catalog>> {
        self.catalog.clone()
    }

    pub fn set_gm_flags(
        &self,
        guid: u64,
        on: Option<bool>,
        visible: Option<bool>,
        chat: Option<bool>,
    ) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&guid) else {
            return;
        };
        if let Some(on) = on {
            presence.player.gm_on = on;
            if on {
                presence.player.gm_chat = true;
            }
        }
        if let Some(chat) = chat {
            presence.player.gm_chat = chat;
        }
        let old_visible = presence.player.gm_visible;
        if let Some(visible) = visible {
            presence.player.gm_visible = visible;
        }
        let player = presence.player.clone();
        if let Some(visible) = visible
            && visible != old_visible
        {
            let event = if visible {
                WorldEvent::PlayerAppeared(player)
            } else {
                WorldEvent::PlayerLeft { guid }
            };
            for other in inner.players.values() {
                if other.player.guid != guid {
                    other.mailbox.send(event.clone());
                }
            }
        }
    }

    pub fn add_item_to_player(&self, guid: u64, item_id: u32, count: u32, stackable: u32) -> bool {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&guid) else {
            return false;
        };
        presence.player.add_item(item_id, count, stackable)
    }

    pub fn set_money(&self, from: &PlayerMailbox, guid: u64, copper: u32) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&guid) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        presence.player.copper = copper;
        from.send(WorldEvent::MoneyChanged { guid, copper });
    }

    pub fn suggest_task(&self, player: u64) -> Option<TaskSpec> {
        let inner = self.inner.lock().expect("world mutex");
        let presence = inner.players.get(&player)?;
        let nearby: Vec<u32> =
            visibility::creatures_in_range(&inner, presence.player.position, CREATE_RANGE)
                .into_iter()
                .filter(|creature| creature.hostile)
                .map(|creature| creature.entry)
                .collect();
        QuestDirector::suggest(1, &nearby)
    }

    pub fn tick(&self, now: Instant) {
        let mut inner = self.inner.lock().expect("world mutex");
        let mut events = Vec::new();
        let mut moved = Vec::new();
        let catalog = self.catalog.clone();
        combat::respawn_creatures(&mut inner, now, &mut events);
        combat::aggro_hostiles(&mut inner, now, &mut events);
        combat::chase_creatures(&mut inner, &mut events, &mut moved);
        combat::leash_creatures(&mut inner, &mut events);
        combat::swing_players(&mut inner, catalog.as_deref(), now, &mut events);
        combat::swing_creatures(&mut inner, now, &mut events);
        combat::regen_players(&mut inner, now, &mut events);
        for event in events {
            visibility::dispatch_event(&mut inner, event);
        }
        for guid in moved {
            visibility::sync_creature_viewers(&mut inner, guid);
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn broadcast(inner: &Inner, event: WorldEvent) {
    for presence in inner.players.values() {
        presence.mailbox.send(event.clone());
    }
}

fn creatures_for_map(map_id: u32) -> Vec<Creature> {
    if map_id == MAP_EASTERN_KINGDOMS {
        northshire_npcs()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
#[path = "../../test/mod.rs"]
mod tests;
