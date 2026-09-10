use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use wow_shared::{MAP_EASTERN_KINGDOMS, Position};
use wow_world_messages::vanilla::opcodes::ServerOpcodeMessage;

use crate::catalog::{Catalog, ItemRow, VendorListing};
use crate::creature::{Creature, Gossip, GossipAction, GossipMenu, northshire_npcs};
use crate::player::{PLAYER_DAMAGE, Player, STAND_STATE_DEAD};
use crate::quest_director::{QuestDirector, TaskSpec};

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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChatChannel {
    Say,
    Yell,
    Emote,
    Whisper { to: String },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Chat {
    pub speaker: u64,
    pub channel: ChatChannel,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChatDelivery {
    Say,
    Yell,
    Emote,
    Whisper,
    WhisperInform,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpokenChat {
    pub from_guid: u64,
    pub text: String,
    pub delivery: ChatDelivery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Attack {
    pub attacker: u64,
    pub victim: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MeleeHit {
    pub attacker: u64,
    pub victim: u64,
    pub damage: u32,
    pub victim_health: i32,
    pub victim_max_health: i32,
    pub victim_dead: bool,
}

#[derive(Clone, Debug)]
pub enum WorldEvent {
    PlayerAppeared(Player),
    PlayerLeft { guid: u64 },
    PlayerMoved(Movement),
    Chat(SpokenChat),
    ChatPlayerNotFound { name: String },
    CreatureAppeared(Creature),
    CreatureLeft { guid: u64 },
    AttackStarted(Attack),
    AttackStopped(Attack),
    MeleeHit(MeleeHit),
    StandStateAck { state: u8 },
    PlayerStandState { guid: u64, state: u8 },
    GossipOpened { npc: u64, menu: GossipMenu },
    GossipClosed,
    VendorOpened { npc: u64, items: Vec<VendorOffer> },
    LootOpened { guid: u64, gold: u32, items: Vec<LootOffer> },
    LootTaken { index: u8 },
    LootClosed { guid: u64 },
    MoneyChanged { guid: u64, copper: u32 },
    CreatureMoved {
        guid: u64,
        from: Position,
        to: Position,
        duration_ms: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VendorOffer {
    pub item_id: u32,
    pub display_id: u32,
    pub max_items: u32,
    pub price: u32,
    pub max_durability: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LootOffer {
    pub index: u8,
    pub item_id: u32,
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

struct Presence {
    player: Player,
    mailbox: PlayerMailbox,
    attacking: Option<u64>,
    next_swing: Instant,
    visible: HashSet<u64>,
    last_cell: (i32, i32),
    looting: Option<u64>,
}

struct CreatureState {
    creature: Creature,
    home: Position,
    combat_target: Option<u64>,
    next_swing: Instant,
    died_at: Option<Instant>,
    damage: i32,
}

struct Inner {
    players: HashMap<u64, Presence>,
    creatures: HashMap<u64, CreatureState>,
    grid: HashMap<(i32, i32), HashSet<u64>>,
    loot: HashMap<u64, Vec<(u32, u32)>>,
}

/// One map's player list, creatures, and combat tick.
#[derive(Clone)]
pub struct World {
    map_id: u32,
    inner: Arc<Mutex<Inner>>,
    catalog: Option<Arc<Catalog>>,
}

impl World {
    pub fn new() -> Self {
        Self::for_map(MAP_EASTERN_KINGDOMS)
    }

    pub fn for_map(map_id: u32) -> Self {
        Self::with_creatures(map_id, creatures_for_map(map_id), None)
    }

    pub fn with_catalog(map_id: u32, catalog: Arc<Catalog>) -> Self {
        let creatures = catalog.creatures_for_map(map_id);
        let creatures = if creatures.is_empty() {
            creatures_for_map(map_id)
        } else {
            creatures
        };
        Self::with_creatures(map_id, creatures, Some(catalog))
    }

    fn with_creatures(map_id: u32, creatures: Vec<Creature>, catalog: Option<Arc<Catalog>>) -> Self {
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
        Self {
            map_id,
            catalog,
            inner: Arc::new(Mutex::new(Inner {
                players: HashMap::new(),
                creatures,
                grid,
                loot: HashMap::new(),
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
            .map(|presence| presence.player.clone())
            .collect();
        broadcast(&inner, WorldEvent::PlayerAppeared(player.clone()));
        let nearby = creatures_in_range(&inner, player.position, CREATE_RANGE);
        let visible: HashSet<u64> = nearby.iter().map(|creature| creature.guid).collect();
        let last_cell = player.position.cell(CELL_SIZE);
        inner.players.insert(
            player.guid,
            Presence {
                player,
                mailbox,
                attacking: None,
                next_swing: Instant::now(),
                visible,
                last_cell,
                looting: None,
            },
        );
        others
    }

    pub fn creatures_near(&self, position: Position) -> Vec<Creature> {
        let inner = self.inner.lock().expect("world mutex");
        creatures_in_range(&inner, position, CREATE_RANGE)
    }

    pub fn leave(&self, guid: u64, mailbox: &PlayerMailbox) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&guid) else {
            return;
        };
        if !presence.mailbox.same_channel(mailbox) {
            return;
        }
        drop_combat_with(&mut inner, guid);
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

    pub fn broadcast_move(&self, from: &PlayerMailbox, movement: Movement) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&movement.guid) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        presence.player.position = movement.position;
        let guid = movement.guid;
        let mailbox = presence.mailbox.clone();
        refresh_visibility(&mut inner, guid, &mailbox);
        let event = WorldEvent::PlayerMoved(movement);
        for (id, presence) in &inner.players {
            if *id != guid {
                presence.mailbox.send(event.clone());
            }
        }
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

    pub fn open_gossip(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let inner = self.inner.lock().expect("world mutex");
        let Some(gossip) = gossip_for(&inner, from, player, npc) else {
            return;
        };
        from.send(WorldEvent::GossipOpened {
            npc,
            menu: gossip.greeting().clone(),
        });
    }

    pub fn select_gossip_option(
        &self,
        from: &PlayerMailbox,
        player: u64,
        npc: u64,
        option_id: u32,
    ) {
        let inner = self.inner.lock().expect("world mutex");
        let Some(gossip) = gossip_for(&inner, from, player, npc) else {
            return;
        };
        let Some(option) = gossip.option(option_id) else {
            return;
        };
        let action = option.action;
        let next_menu = match action {
            GossipAction::ShowMenu { text_id } => gossip.menu(text_id).cloned(),
            _ => None,
        };
        match action {
            GossipAction::Close => from.send(WorldEvent::GossipClosed),
            GossipAction::ShowMenu { .. } => {
                let Some(menu) = next_menu else {
                    return;
                };
                from.send(WorldEvent::GossipOpened { npc, menu });
            }
            GossipAction::OpenVendor => {
                drop(inner);
                self.list_vendor(from, player, npc);
            }
        }
    }

    pub fn list_vendor(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let Some(creature) = inner.creatures.get(&npc) else {
            return;
        };
        if creature.creature.npc_flags & crate::creature::NPC_FLAG_VENDOR == 0 {
            return;
        }
        if !in_range(
            presence.player.position,
            creature.creature.position,
            GOSSIP_RANGE,
        ) {
            return;
        }
        let entry = creature.creature.entry;
        let listings = self
            .catalog
            .as_ref()
            .map(|catalog| catalog.vendor_listings(entry))
            .unwrap_or_default();
        let items = listings
            .into_iter()
            .filter_map(|listing| vendor_offer(self.catalog.as_deref(), listing))
            .collect();
        from.send(WorldEvent::GossipClosed);
        from.send(WorldEvent::VendorOpened { npc, items });
    }

    pub fn buy_item(&self, from: &PlayerMailbox, player: u64, npc: u64, item_id: u32, amount: u32) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let Some(creature) = inner.creatures.get(&npc) else {
            return;
        };
        if creature.creature.npc_flags & crate::creature::NPC_FLAG_VENDOR == 0 {
            return;
        }
        let entry = creature.creature.entry;
        let Some(catalog) = self.catalog.as_ref() else {
            return;
        };
        let listings = catalog.vendor_listings(entry);
        if !listings.iter().any(|listing| listing.item_id == item_id) {
            return;
        }
        let Some(item) = catalog.item(item_id) else {
            return;
        };
        let amount = amount.max(1);
        let price = item.buy_price.saturating_mul(amount);
        let stackable = item.stackable;
        let presence = inner.players.get_mut(&player).expect("player");
        if presence.player.copper < price {
            return;
        }
        if !presence.player.add_item(item_id, amount, stackable) {
            return;
        }
        presence.player.copper -= price;
        let copper = presence.player.copper;
        from.send(WorldEvent::MoneyChanged {
            guid: player,
            copper,
        });
    }

    pub fn open_loot(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let Some(creature) = inner.creatures.get(&npc) else {
            return;
        };
        if !creature.creature.dead
            || !in_range(
                presence.player.position,
                creature.creature.position,
                GOSSIP_RANGE,
            )
        {
            return;
        }
        let loot_id = creature.creature.loot_id;
        if !inner.loot.contains_key(&npc) {
            let drops = self
                .catalog
                .as_ref()
                .map(|catalog| catalog.roll_loot(loot_id))
                .unwrap_or_default();
            inner.loot.insert(npc, drops);
        }
        let items = inner
            .loot
            .get(&npc)
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(_, (item_id, _))| *item_id != 0)
            .map(|(index, (item_id, _))| LootOffer {
                index: index as u8,
                item_id: *item_id,
            })
            .collect();
        from.send(WorldEvent::LootOpened {
            guid: npc,
            gold: 0,
            items,
        });
        if let Some(presence) = inner.players.get_mut(&player) {
            presence.looting = Some(npc);
        }
    }

    pub fn take_loot(&self, from: &PlayerMailbox, player: u64, index: u8) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let Some(npc) = presence.looting else {
            return;
        };
        let Some(drops) = inner.loot.get(&npc).cloned() else {
            return;
        };
        if index as usize >= drops.len() {
            return;
        }
        let (item_id, count) = drops[index as usize];
        if item_id == 0 {
            return;
        }
        let stackable = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.item(item_id))
            .map(|item| item.stackable)
            .unwrap_or(1);
        let presence = inner.players.get_mut(&player).expect("player");
        if !presence.player.add_item(item_id, count, stackable) {
            return;
        }
        if let Some(drops) = inner.loot.get_mut(&npc) {
            drops[index as usize] = (0, 0);
        }
        from.send(WorldEvent::LootTaken { index });
    }

    pub fn close_loot(&self, from: &PlayerMailbox, player: u64) {
        let mut inner = self.inner.lock().expect("world mutex");
        if let Some(presence) = inner.players.get_mut(&player) {
            if presence.mailbox.same_channel(from) {
                let guid = presence.looting.take().unwrap_or(0);
                from.send(WorldEvent::LootClosed { guid });
            }
        }
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

    pub fn item(&self, entry: u32) -> Option<ItemRow> {
        self.catalog
            .as_ref()
            .and_then(|catalog| catalog.item(entry).cloned())
    }

    pub fn suggest_task(&self, player: u64) -> Option<TaskSpec> {
        let inner = self.inner.lock().expect("world mutex");
        let presence = inner.players.get(&player)?;
        let nearby: Vec<u32> = creatures_in_range(&inner, presence.player.position, CREATE_RANGE)
            .into_iter()
            .filter(|creature| creature.hostile)
            .map(|creature| creature.entry)
            .collect();
        QuestDirector::suggest(1, &nearby)
    }

    pub fn speak(&self, from: &PlayerMailbox, chat: Chat) {
        let inner = self.inner.lock().expect("world mutex");
        let Some(speaker) = inner.players.get(&chat.speaker) else {
            return;
        };
        if !speaker.mailbox.same_channel(from) {
            return;
        }
        let origin = speaker.player.position;

        match chat.channel {
            ChatChannel::Say => {
                broadcast_in_range(
                    &inner,
                    origin,
                    SAY_RANGE,
                    SpokenChat {
                        from_guid: chat.speaker,
                        text: chat.text,
                        delivery: ChatDelivery::Say,
                    },
                );
            }
            ChatChannel::Yell => {
                broadcast_in_range(
                    &inner,
                    origin,
                    YELL_RANGE,
                    SpokenChat {
                        from_guid: chat.speaker,
                        text: chat.text,
                        delivery: ChatDelivery::Yell,
                    },
                );
            }
            ChatChannel::Emote => {
                broadcast_in_range(
                    &inner,
                    origin,
                    EMOTE_RANGE,
                    SpokenChat {
                        from_guid: chat.speaker,
                        text: chat.text,
                        delivery: ChatDelivery::Emote,
                    },
                );
            }
            ChatChannel::Whisper { to } => {
                let target = inner
                    .players
                    .values()
                    .find(|presence| presence.player.name.eq_ignore_ascii_case(&to));
                let Some(target) = target else {
                    from.send(WorldEvent::ChatPlayerNotFound { name: to });
                    return;
                };
                target.mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: chat.speaker,
                    text: chat.text.clone(),
                    delivery: ChatDelivery::Whisper,
                }));
                from.send(WorldEvent::Chat(SpokenChat {
                    from_guid: target.player.guid,
                    text: chat.text,
                    delivery: ChatDelivery::WhisperInform,
                }));
            }
        }
    }

    pub fn start_attack(&self, from: &PlayerMailbox, attacker: u64, target: u64) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&attacker) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        if !presence.player.is_alive() {
            from.send(WorldEvent::AttackStopped(Attack {
                attacker,
                victim: target,
            }));
            return;
        }

        let Some(creature) = inner.creatures.get(&target) else {
            from.send(WorldEvent::AttackStopped(Attack {
                attacker,
                victim: target,
            }));
            return;
        };
        if !creature.creature.hostile || creature.creature.dead {
            from.send(WorldEvent::AttackStopped(Attack {
                attacker,
                victim: target,
            }));
            return;
        }

        let now = Instant::now();
        if let Some(presence) = inner.players.get_mut(&attacker) {
            presence.attacking = Some(target);
            presence.next_swing = now;
        }
        if let Some(creature) = inner.creatures.get_mut(&target) {
            if creature.combat_target.is_none() {
                creature.combat_target = Some(attacker);
                creature.next_swing = now;
            }
        }
        broadcast(
            &inner,
            WorldEvent::AttackStarted(Attack {
                attacker,
                victim: target,
            }),
        );
    }

    pub fn stop_attack(&self, from: &PlayerMailbox, attacker: u64) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&attacker) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let victim = presence.attacking.unwrap_or(0);
        if let Some(presence) = inner.players.get_mut(&attacker) {
            presence.attacking = None;
        }
        broadcast(
            &inner,
            WorldEvent::AttackStopped(Attack { attacker, victim }),
        );
    }

    pub fn tick(&self, now: Instant) {
        let mut inner = self.inner.lock().expect("world mutex");
        let mut events = Vec::new();
        let mut moved = Vec::new();
        respawn_creatures(&mut inner, now, &mut events);
        aggro_hostiles(&mut inner, now, &mut events);
        chase_creatures(&mut inner, &mut events, &mut moved);
        leash_creatures(&mut inner, &mut events);
        swing_players(&mut inner, now, &mut events);
        swing_creatures(&mut inner, now, &mut events);
        for event in events {
            dispatch_event(&mut inner, event);
        }
        for guid in moved {
            sync_creature_viewers(&mut inner, guid);
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

const SAY_RANGE: f32 = 25.0;
const YELL_RANGE: f32 = 300.0;
const EMOTE_RANGE: f32 = 25.0;
const MELEE_RANGE: f32 = 5.0;
const AGGRO_RANGE: f32 = 18.0;
const LEASH_RANGE: f32 = 50.0;
const GOSSIP_RANGE: f32 = 10.0;
const SWING_INTERVAL: Duration = Duration::from_millis(2000);
const CELL_SIZE: f32 = 40.0;
const CREATE_RANGE: f32 = 90.0;
const DESTROY_RANGE: f32 = 100.0;
const CELL_HYSTERESIS: f32 = 10.0;
const CREATURE_SPEED: f32 = 7.0;
const TICK_SECS: f32 = 0.2;

fn vendor_offer(catalog: Option<&Catalog>, listing: VendorListing) -> Option<VendorOffer> {
    let item = catalog?.item(listing.item_id)?;
    Some(VendorOffer {
        item_id: listing.item_id,
        display_id: item.display_id,
        max_items: if listing.maxcount <= 0 {
            u32::MAX
        } else {
            listing.maxcount as u32
        },
        price: item.buy_price,
        max_durability: item.max_durability,
    })
}

fn creatures_for_map(map_id: u32) -> Vec<Creature> {
    if map_id == MAP_EASTERN_KINGDOMS {
        northshire_npcs()
    } else {
        Vec::new()
    }
}

fn broadcast_in_range(inner: &Inner, origin: Position, range: f32, spoken: SpokenChat) {
    let range_squared = range * range;
    let event = WorldEvent::Chat(spoken);
    for presence in inner.players.values() {
        if presence.player.position.distance_squared(origin) <= range_squared {
            presence.mailbox.send(event.clone());
        }
    }
}

fn broadcast(inner: &Inner, event: WorldEvent) {
    for presence in inner.players.values() {
        presence.mailbox.send(event.clone());
    }
}

fn dispatch_event(inner: &mut Inner, event: WorldEvent) {
    match &event {
        WorldEvent::CreatureAppeared(creature) => {
            for presence in inner.players.values_mut() {
                if in_range(presence.player.position, creature.position, CREATE_RANGE) {
                    presence.visible.insert(creature.guid);
                    presence.mailbox.send(event.clone());
                }
            }
        }
        WorldEvent::CreatureLeft { guid } => {
            for presence in inner.players.values_mut() {
                if presence.visible.remove(guid) {
                    presence.mailbox.send(event.clone());
                }
            }
        }
        WorldEvent::CreatureMoved { guid, .. } => {
            for presence in inner.players.values() {
                if presence.visible.contains(guid) {
                    presence.mailbox.send(event.clone());
                }
            }
        }
        _ => broadcast(inner, event),
    }
}

fn creatures_in_range(inner: &Inner, origin: Position, range: f32) -> Vec<Creature> {
    let range_sq = range * range;
    let cell = origin.cell(CELL_SIZE);
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(bucket) = inner.grid.get(&(cell.0 + dx, cell.1 + dy)) else {
                continue;
            };
            for guid in bucket {
                if !seen.insert(*guid) {
                    continue;
                }
                let Some(state) = inner.creatures.get(guid) else {
                    continue;
                };
                if origin.distance_squared(state.creature.position) <= range_sq {
                    out.push(state.creature.clone());
                }
            }
        }
    }
    out
}

fn refresh_visibility(inner: &mut Inner, player: u64, mailbox: &PlayerMailbox) {
    let Some(presence) = inner.players.get(&player) else {
        return;
    };
    let position = presence.player.position;
    let last_cell = presence.last_cell;
    let next_cell = sticky_cell(position, last_cell);
    if next_cell == last_cell {
        return;
    }
    let current = presence.visible.clone();
    let nearby: HashSet<u64> = creatures_in_range(inner, position, CREATE_RANGE)
        .into_iter()
        .map(|creature| creature.guid)
        .collect();
    let destroy_sq = DESTROY_RANGE * DESTROY_RANGE;
    let mut next = HashSet::new();
    for guid in current.iter().copied() {
        let Some(state) = inner.creatures.get(&guid) else {
            mailbox.send(WorldEvent::CreatureLeft { guid });
            continue;
        };
        if position.distance_squared(state.creature.position) <= destroy_sq {
            next.insert(guid);
        } else {
            mailbox.send(WorldEvent::CreatureLeft { guid });
        }
    }
    for guid in nearby {
        if next.insert(guid)
            && let Some(state) = inner.creatures.get(&guid)
        {
            mailbox.send(WorldEvent::CreatureAppeared(state.creature.clone()));
        }
    }
    if let Some(presence) = inner.players.get_mut(&player) {
        presence.visible = next;
        presence.last_cell = next_cell;
    }
}

fn sticky_cell(position: Position, last: (i32, i32)) -> (i32, i32) {
    let fresh = position.cell(CELL_SIZE);
    if fresh == last {
        return last;
    }
    let cx = (last.0 as f32 + 0.5) * CELL_SIZE;
    let cy = (last.1 as f32 + 0.5) * CELL_SIZE;
    let limit = CELL_SIZE / 2.0 + CELL_HYSTERESIS;
    if (position.x - cx).abs() <= limit && (position.y - cy).abs() <= limit {
        last
    } else {
        fresh
    }
}

fn move_grid(
    grid: &mut HashMap<(i32, i32), HashSet<u64>>,
    guid: u64,
    from: (i32, i32),
    to: (i32, i32),
) {
    if from == to {
        return;
    }
    if let Some(bucket) = grid.get_mut(&from) {
        bucket.remove(&guid);
        if bucket.is_empty() {
            grid.remove(&from);
        }
    }
    grid.entry(to).or_default().insert(guid);
}

fn chase_creatures(inner: &mut Inner, events: &mut Vec<WorldEvent>, moved: &mut Vec<u64>) {
    let mut moves = Vec::new();
    let targets: Vec<(u64, u64, Position, Position)> = inner
        .creatures
        .values()
        .filter(|state| !state.creature.dead)
        .filter_map(|state| {
            let target = state.combat_target?;
            let player = inner.players.get(&target)?;
            if in_range(
                state.creature.position,
                player.player.position,
                MELEE_RANGE,
            ) {
                return None;
            }
            Some((
                state.creature.guid,
                target,
                state.creature.position,
                player.player.position,
            ))
        })
        .collect();
    for (guid, _, from, dest) in targets {
        let dx = dest.x - from.x;
        let dy = dest.y - from.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        let step = CREATURE_SPEED * TICK_SECS;
        let ratio = (step / dist).min(1.0);
        let to = Position {
            x: from.x + dx * ratio,
            y: from.y + dy * ratio,
            z: dest.z,
            orientation: dy.atan2(dx),
        };
        let (old_cell, new_cell) = if let Some(state) = inner.creatures.get_mut(&guid) {
            let old_cell = state.creature.position.cell(CELL_SIZE);
            state.creature.position = to;
            (old_cell, to.cell(CELL_SIZE))
        } else {
            continue;
        };
        move_grid(&mut inner.grid, guid, old_cell, new_cell);
        let duration_ms = ((dist / CREATURE_SPEED) * 1000.0).clamp(50.0, 1000.0) as u32;
        moved.push(guid);
        moves.push(WorldEvent::CreatureMoved {
            guid,
            from,
            to,
            duration_ms,
        });
    }
    events.extend(moves);
}

fn sync_creature_viewers(inner: &mut Inner, guid: u64) {
    let Some(creature) = inner
        .creatures
        .get(&guid)
        .map(|state| state.creature.clone())
    else {
        return;
    };
    let position = creature.position;
    let create_sq = CREATE_RANGE * CREATE_RANGE;
    let destroy_sq = DESTROY_RANGE * DESTROY_RANGE;
    for presence in inner.players.values_mut() {
        let dist = presence.player.position.distance_squared(position);
        let seen = presence.visible.contains(&guid);
        if seen && dist > destroy_sq {
            presence.visible.remove(&guid);
            presence.mailbox.send(WorldEvent::CreatureLeft { guid });
        } else if !seen && dist <= create_sq {
            presence.visible.insert(guid);
            presence
                .mailbox
                .send(WorldEvent::CreatureAppeared(creature.clone()));
        }
    }
}

fn in_range(a: Position, b: Position, range: f32) -> bool {
    a.distance_squared(b) <= range * range
}

fn gossip_for<'a>(
    inner: &'a Inner,
    from: &PlayerMailbox,
    player: u64,
    npc: u64,
) -> Option<&'a Gossip> {
    let presence = inner.players.get(&player)?;
    if !presence.mailbox.same_channel(from) {
        return None;
    }
    let creature = inner.creatures.get(&npc)?;
    if !creature.creature.can_gossip() {
        return None;
    }
    if !in_range(
        presence.player.position,
        creature.creature.position,
        GOSSIP_RANGE,
    ) {
        return None;
    }
    creature.creature.gossip.as_ref()
}

fn respawn_creatures(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
    let mut respawn = Vec::new();
    for state in inner.creatures.values() {
        let Some(died_at) = state.died_at else {
            continue;
        };
        if now.duration_since(died_at) >= Duration::from_secs(state.creature.respawn_secs as u64) {
            respawn.push(state.creature.guid);
        }
    }
    for guid in respawn {
        let Some((old_cell, home, appeared)) = inner.creatures.get_mut(&guid).map(|state| {
            let old_cell = state.creature.position.cell(CELL_SIZE);
            state.creature.health = state.creature.max_health;
            state.creature.dead = false;
            state.creature.position = state.home;
            state.combat_target = None;
            state.died_at = None;
            state.next_swing = now;
            (old_cell, state.home, state.creature.clone())
        }) else {
            continue;
        };
        inner.loot.remove(&guid);
        move_grid(&mut inner.grid, guid, old_cell, home.cell(CELL_SIZE));
        events.push(WorldEvent::CreatureLeft { guid });
        events.push(WorldEvent::CreatureAppeared(appeared));
    }
}

fn aggro_hostiles(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
    let players: Vec<(u64, Position)> = inner
        .players
        .values()
        .filter(|presence| presence.player.is_alive())
        .map(|presence| (presence.player.guid, presence.player.position))
        .collect();

    let mut starts = Vec::new();
    for state in inner.creatures.values_mut() {
        if !state.creature.hostile || state.creature.dead || state.combat_target.is_some() {
            continue;
        }
        let target = players
            .iter()
            .filter(|(_, position)| in_range(state.creature.position, *position, AGGRO_RANGE))
            .min_by(|a, b| {
                state
                    .creature
                    .position
                    .distance_squared(a.1)
                    .total_cmp(&state.creature.position.distance_squared(b.1))
            });
        let Some((player_guid, _)) = target else {
            continue;
        };
        state.combat_target = Some(*player_guid);
        state.next_swing = now;
        starts.push(Attack {
            attacker: state.creature.guid,
            victim: *player_guid,
        });
    }
    events.extend(starts.into_iter().map(WorldEvent::AttackStarted));
}

fn leash_creatures(inner: &mut Inner, events: &mut Vec<WorldEvent>) {
    let mut stops = Vec::new();
    let mut reset = Vec::new();
    for state in inner.creatures.values() {
        let Some(target) = state.combat_target else {
            continue;
        };
        let Some(player) = inner.players.get(&target) else {
            reset.push(state.creature.guid);
            stops.push(Attack {
                attacker: state.creature.guid,
                victim: target,
            });
            continue;
        };
        if !player.player.is_alive() || !in_range(state.home, player.player.position, LEASH_RANGE) {
            reset.push(state.creature.guid);
            stops.push(Attack {
                attacker: state.creature.guid,
                victim: target,
            });
        }
    }
    for guid in reset {
        if let Some(state) = inner.creatures.get_mut(&guid) {
            state.combat_target = None;
        }
    }
    events.extend(stops.into_iter().map(WorldEvent::AttackStopped));
}

fn swing_players(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
    let swings: Vec<(u64, u64)> = inner
        .players
        .values()
        .filter(|presence| presence.player.is_alive() && now >= presence.next_swing)
        .filter_map(|presence| {
            presence
                .attacking
                .map(|target| (presence.player.guid, target))
        })
        .collect();

    for (attacker, victim) in swings {
        let Some(player) = inner.players.get(&attacker) else {
            continue;
        };
        let origin = player.player.position;
        let Some(creature) = inner.creatures.get(&victim) else {
            continue;
        };
        if creature.creature.dead || !in_range(origin, creature.creature.position, MELEE_RANGE) {
            continue;
        }
        if let Some(player) = inner.players.get_mut(&attacker) {
            player.next_swing = now + SWING_INTERVAL;
        }
        land_hit(inner, attacker, victim, PLAYER_DAMAGE, events);
    }
}

fn swing_creatures(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
    let swings: Vec<(u64, u64, i32)> = inner
        .creatures
        .values()
        .filter(|state| {
            !state.creature.dead && state.combat_target.is_some() && now >= state.next_swing
        })
        .filter_map(|state| {
            state
                .combat_target
                .map(|target| (state.creature.guid, target, state.damage))
        })
        .collect();

    for (attacker, victim, damage) in swings {
        let Some(creature) = inner.creatures.get(&attacker) else {
            continue;
        };
        let origin = creature.creature.position;
        let Some(player) = inner.players.get(&victim) else {
            continue;
        };
        if !player.player.is_alive() || !in_range(origin, player.player.position, MELEE_RANGE) {
            continue;
        }
        if let Some(creature) = inner.creatures.get_mut(&attacker) {
            creature.next_swing = now + SWING_INTERVAL;
        }
        land_hit(inner, attacker, victim, damage, events);
    }
}

fn land_hit(
    inner: &mut Inner,
    attacker: u64,
    victim: u64,
    damage: i32,
    events: &mut Vec<WorldEvent>,
) {
    let Some(hit) = apply_damage(inner, attacker, victim, damage) else {
        return;
    };
    let victim_dead = hit.victim_dead;
    events.push(WorldEvent::MeleeHit(hit));
    if victim_dead {
        end_combat_for(inner, victim, events);
    }
}

fn apply_damage(inner: &mut Inner, attacker: u64, victim: u64, damage: i32) -> Option<MeleeHit> {
    if let Some(creature) = inner.creatures.get_mut(&victim) {
        if creature.creature.dead {
            return None;
        }
        creature.creature.health = (creature.creature.health - damage).max(0);
        let dead = creature.creature.health == 0;
        creature.creature.dead = dead;
        if dead {
            creature.died_at = Some(Instant::now());
            creature.combat_target = None;
        }
        return Some(MeleeHit {
            attacker,
            victim,
            damage: damage as u32,
            victim_health: creature.creature.health,
            victim_max_health: creature.creature.max_health,
            victim_dead: dead,
        });
    }

    let player = inner.players.get_mut(&victim)?;
    if !player.player.is_alive() {
        return None;
    }
    player.player.health = (player.player.health - damage).max(0);
    let dead = !player.player.is_alive();
    if dead {
        player.player.stand_state = STAND_STATE_DEAD;
    }
    Some(MeleeHit {
        attacker,
        victim,
        damage: damage as u32,
        victim_health: player.player.health,
        victim_max_health: player.player.max_health,
        victim_dead: dead,
    })
}

fn end_combat_for(inner: &mut Inner, guid: u64, events: &mut Vec<WorldEvent>) {
    let mut stops = Vec::new();
    for presence in inner.players.values_mut() {
        if presence.attacking == Some(guid) {
            presence.attacking = None;
            stops.push(Attack {
                attacker: presence.player.guid,
                victim: guid,
            });
        }
    }
    for state in inner.creatures.values_mut() {
        if state.combat_target == Some(guid) {
            state.combat_target = None;
            stops.push(Attack {
                attacker: state.creature.guid,
                victim: guid,
            });
        }
        if state.creature.guid == guid {
            if let Some(target) = state.combat_target.take() {
                stops.push(Attack {
                    attacker: guid,
                    victim: target,
                });
            }
        }
    }
    events.extend(stops.into_iter().map(WorldEvent::AttackStopped));
}

fn drop_combat_with(inner: &mut Inner, player_guid: u64) {
    for state in inner.creatures.values_mut() {
        if state.combat_target == Some(player_guid) {
            state.combat_target = None;
        }
    }
}

#[cfg(test)]
#[path = "../test/mod.rs"]
mod tests;
