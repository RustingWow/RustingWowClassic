use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use wow_shared::Position;
use wow_world_messages::vanilla::opcodes::ServerOpcodeMessage;

use crate::creature::{northshire_npcs, Creature, Gossip, GossipAction, GossipMenu, WOLF_DAMAGE};
use crate::player::{Player, PLAYER_DAMAGE, STAND_STATE_DEAD};

/// Movement already translated for nearby clients; the packet body stays private.
#[derive(Clone, Debug)]
pub struct Movement {
    pub guid: u64,
    pub position: Position,
    packet: ServerOpcodeMessage,
}

impl Movement {
    pub(crate) fn new(guid: u64, position: Position, packet: ServerOpcodeMessage) -> Self {
        Self {
            guid,
            position,
            packet,
        }
    }

    pub(crate) fn packet(&self) -> &ServerOpcodeMessage {
        &self.packet
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatChannel {
    Say,
    Yell,
    Emote,
    Whisper { to: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chat {
    pub speaker: u64,
    pub channel: ChatChannel,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatDelivery {
    Say,
    Yell,
    Emote,
    Whisper,
    WhisperInform,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenChat {
    pub from_guid: u64,
    pub text: String,
    pub delivery: ChatDelivery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attack {
    pub attacker: u64,
    pub victim: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    fn send(&self, event: WorldEvent) {
        let _ = self.tx.send(event);
    }

    fn same_channel(&self, other: &Self) -> bool {
        self.tx.same_channel(&other.tx)
    }
}

struct Presence {
    player: Player,
    mailbox: PlayerMailbox,
    attacking: Option<u64>,
    next_swing: Instant,
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
}

/// Shared in-world player list. Sessions hear about each other through mailboxes.
#[derive(Clone)]
pub struct World {
    inner: Arc<Mutex<Inner>>,
}

impl World {
    pub fn new() -> Self {
        let now = Instant::now();
        let creatures = northshire_npcs()
            .into_iter()
            .map(|creature| {
                let damage = if creature.hostile { WOLF_DAMAGE } else { 0 };
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
            inner: Arc::new(Mutex::new(Inner {
                players: HashMap::new(),
                creatures,
            })),
        }
    }

    pub fn join(&self, player: Player, mailbox: PlayerMailbox) -> Vec<Player> {
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
        inner.players.insert(
            player.guid,
            Presence {
                player,
                mailbox,
                attacking: None,
                next_swing: Instant::now(),
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
        if let Some(presence) = inner.players.get_mut(&movement.guid) {
            if !presence.mailbox.same_channel(from) {
                return;
            }
            presence.player.position = movement.position;
        }
        let guid = movement.guid;
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
        match option.action {
            GossipAction::Close => from.send(WorldEvent::GossipClosed),
            GossipAction::ShowMenu { text_id } => {
                let Some(menu) = gossip.menu(text_id) else {
                    return;
                };
                from.send(WorldEvent::GossipOpened {
                    npc,
                    menu: menu.clone(),
                });
            }
        }
    }

    pub fn npc_text(&self, text_id: u32) -> Option<String> {
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
        respawn_creatures(&mut inner, now, &mut events);
        aggro_hostiles(&mut inner, now, &mut events);
        leash_creatures(&mut inner, &mut events);
        swing_players(&mut inner, now, &mut events);
        swing_creatures(&mut inner, now, &mut events);
        for event in events {
            broadcast(&inner, event);
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
const RESPAWN_AFTER: Duration = Duration::from_secs(20);

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
        if now.duration_since(died_at) >= RESPAWN_AFTER {
            respawn.push(state.creature.guid);
        }
    }
    for guid in respawn {
        let Some(state) = inner.creatures.get_mut(&guid) else {
            continue;
        };
        state.creature.health = state.creature.max_health;
        state.creature.dead = false;
        state.creature.position = state.home;
        state.combat_target = None;
        state.died_at = None;
        state.next_swing = now;
        events.push(WorldEvent::CreatureLeft { guid });
        events.push(WorldEvent::CreatureAppeared(state.creature.clone()));
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
        if !player.player.is_alive()
            || !in_range(state.home, player.player.position, LEASH_RANGE)
        {
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

fn apply_damage(
    inner: &mut Inner,
    attacker: u64,
    victim: u64,
    damage: i32,
) -> Option<MeleeHit> {
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

pub struct WorldPresence {
    world: World,
    guid: u64,
    mailbox: PlayerMailbox,
}

impl WorldPresence {
    pub fn new(world: World, guid: u64, mailbox: PlayerMailbox) -> Self {
        Self {
            world,
            guid,
            mailbox,
        }
    }
}

impl Drop for WorldPresence {
    fn drop(&mut self) {
        self.world.leave(self.guid, &self.mailbox);
    }
}

#[cfg(test)]
#[path = "../test/mod.rs"]
mod tests;
