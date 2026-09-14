use std::time::{Duration, Instant};

use wow_shared::Position;

use crate::catalog::Catalog;
use crate::player::{PLAYER_DAMAGE, STAND_STATE_DEAD, STAND_STATE_SIT};

use super::quest::credit_quest_kills;
use super::visibility::{in_range, move_grid};
use super::{
    AGGRO_RANGE, CELL_SIZE, COMBAT_REGEN_DELAY, CREATURE_SPEED, Inner, LEASH_RANGE, MELEE_RANGE,
    PlayerMailbox, REGEN_INTERVAL, SWING_INTERVAL, TICK_SECS, World, WorldEvent, broadcast,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Attack {
    pub attacker: u64,
    pub victim: u64,
    pub victim_dead: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MeleeHit {
    pub attacker: u64,
    pub victim: u64,
    pub damage: u32,
    pub victim_health: i32,
    pub victim_max_health: i32,
    pub victim_dead: bool,
    pub lootable: bool,
}

impl World {
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
                victim_dead: false,
            }));
            return;
        }

        let Some(creature) = inner.creatures.get(&target) else {
            from.send(WorldEvent::AttackStopped(Attack {
                attacker,
                victim: target,
                victim_dead: false,
            }));
            return;
        };
        let hostile = creature.creature.hostile;
        let dead = creature.creature.dead;
        if !hostile {
            from.send(WorldEvent::AttackStopped(Attack {
                attacker,
                victim: target,
                victim_dead: false,
            }));
            return;
        }
        if dead {
            drop(inner);
            self.open_loot(from, attacker, target);
            return;
        }

        let now = Instant::now();
        if let Some(presence) = inner.players.get_mut(&attacker) {
            presence.attacking = Some(target);
            presence.next_swing = now;
            presence.last_combat = Some(now);
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
                victim_dead: false,
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
            WorldEvent::AttackStopped(Attack {
                attacker,
                victim,
                victim_dead: false,
            }),
        );
    }

    pub fn set_health(&self, guid: u64, health: i32, dead: bool) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&guid) else {
            return;
        };
        presence.player.health = health.max(0);
        if dead {
            presence.player.health = 0;
            presence.player.stand_state = STAND_STATE_DEAD;
        } else {
            presence.player.health = presence.player.health.max(1);
            presence.player.stand_state = crate::player::STAND_STATE_STAND;
        }
        let mailbox = presence.mailbox.clone();
        let health = presence.player.health;
        let max_health = presence.player.max_health;
        let victim_dead = !presence.player.is_alive();
        mailbox.send(WorldEvent::MeleeHit(MeleeHit {
            attacker: guid,
            victim: guid,
            damage: 0,
            victim_health: health,
            victim_max_health: max_health,
            victim_dead,
            lootable: false,
        }));
        mailbox.send(WorldEvent::PlayerStandState {
            guid,
            state: presence.player.stand_state,
        });
    }
}

pub(crate) fn regen_players(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
    let targeted: std::collections::HashSet<u64> = inner
        .creatures
        .values()
        .filter_map(|state| state.combat_target)
        .collect();
    let mut hits = Vec::new();
    for presence in inner.players.values_mut() {
        if now < presence.next_regen {
            continue;
        }
        presence.next_regen = now + REGEN_INTERVAL;
        if !presence.player.is_alive() || presence.player.health >= presence.player.max_health {
            continue;
        }
        if presence.attacking.is_some() || targeted.contains(&presence.player.guid) {
            continue;
        }
        if presence
            .last_combat
            .is_some_and(|at| now < at + COMBAT_REGEN_DELAY)
        {
            continue;
        }
        let amount = health_regen_amount(presence.player.max_health, presence.player.stand_state);
        presence.player.health = (presence.player.health + amount).min(presence.player.max_health);
        hits.push(MeleeHit {
            attacker: presence.player.guid,
            victim: presence.player.guid,
            damage: 0,
            victim_health: presence.player.health,
            victim_max_health: presence.player.max_health,
            victim_dead: false,
            lootable: false,
        });
    }
    events.extend(hits.into_iter().map(WorldEvent::MeleeHit));
}

pub(crate) fn health_regen_amount(max_health: i32, stand_state: u8) -> i32 {
    let base = (max_health / 20).max(1);
    if stand_state == STAND_STATE_SIT {
        base * 2
    } else {
        base
    }
}

pub(crate) fn drop_combat_with(inner: &mut Inner, player_guid: u64) {
    for state in inner.creatures.values_mut() {
        if state.combat_target == Some(player_guid) {
            state.combat_target = None;
        }
    }
}

pub(crate) fn respawn_creatures(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
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
            state.creature.lootable = false;
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

pub(crate) fn aggro_hostiles(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
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
            victim_dead: false,
        });
    }
    events.extend(starts.into_iter().map(WorldEvent::AttackStarted));
}

pub(crate) fn chase_creatures(
    inner: &mut Inner,
    events: &mut Vec<WorldEvent>,
    moved: &mut Vec<u64>,
) {
    let mut moves = Vec::new();
    let targets: Vec<(u64, u64, Position, Position)> = inner
        .creatures
        .values()
        .filter(|state| !state.creature.dead)
        .filter_map(|state| {
            let target = state.combat_target?;
            let player = inner.players.get(&target)?;
            if in_range(state.creature.position, player.player.position, MELEE_RANGE) {
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

pub(crate) fn leash_creatures(inner: &mut Inner, events: &mut Vec<WorldEvent>) {
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
                victim_dead: false,
            });
            continue;
        };
        if !player.player.is_alive() || !in_range(state.home, player.player.position, LEASH_RANGE) {
            reset.push(state.creature.guid);
            stops.push(Attack {
                attacker: state.creature.guid,
                victim: target,
                victim_dead: false,
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

pub(crate) fn swing_players(
    inner: &mut Inner,
    catalog: Option<&Catalog>,
    now: Instant,
    events: &mut Vec<WorldEvent>,
) {
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
        land_hit(inner, catalog, attacker, victim, PLAYER_DAMAGE, events);
    }
}

pub(crate) fn swing_creatures(inner: &mut Inner, now: Instant, events: &mut Vec<WorldEvent>) {
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
        land_hit(inner, None, attacker, victim, damage, events);
    }
}

fn land_hit(
    inner: &mut Inner,
    catalog: Option<&Catalog>,
    attacker: u64,
    victim: u64,
    damage: i32,
    events: &mut Vec<WorldEvent>,
) {
    let Some(mut hit) = apply_damage(inner, attacker, victim, damage) else {
        return;
    };
    let victim_dead = hit.victim_dead;
    if victim_dead && inner.creatures.contains_key(&victim) {
        super::vendor::fill_creature_loot(inner, catalog, victim);
        hit.lootable = inner
            .creatures
            .get(&victim)
            .is_some_and(|state| state.creature.lootable);
    }
    events.push(WorldEvent::MeleeHit(hit));
    if victim_dead {
        if let Some(catalog) = catalog {
            credit_quest_kills(inner, catalog, attacker, victim, events);
        }
        end_combat_for(inner, victim, events);
        if inner.creatures.contains_key(&victim) {
            if let Some(state) = inner.creatures.get(&victim) {
                events.push(WorldEvent::CreatureLeft { guid: victim });
                events.push(WorldEvent::CreatureAppeared(state.creature.clone()));
            }
            if let Some(event) =
                super::vendor::offer_creature_loot(inner, catalog, attacker, victim)
            {
                events.push(event);
            }
        }
    }
}

fn apply_damage(inner: &mut Inner, attacker: u64, victim: u64, damage: i32) -> Option<MeleeHit> {
    let now = Instant::now();
    if inner.creatures.contains_key(&victim) {
        let hit = {
            let creature = inner.creatures.get_mut(&victim)?;
            if creature.creature.dead {
                return None;
            }
            creature.creature.health = (creature.creature.health - damage).max(0);
            let dead = creature.creature.health == 0;
            creature.creature.dead = dead;
            if dead {
                creature.died_at = Some(now);
                creature.combat_target = None;
            }
            MeleeHit {
                attacker,
                victim,
                damage: damage as u32,
                victim_health: creature.creature.health,
                victim_max_health: creature.creature.max_health,
                victim_dead: dead,
                lootable: false,
            }
        };
        mark_combat(inner, attacker, now);
        return Some(hit);
    }

    let hit = {
        let player = inner.players.get_mut(&victim)?;
        if !player.player.is_alive() || player.player.gm_on {
            return None;
        }
        player.player.health = (player.player.health - damage).max(0);
        let dead = !player.player.is_alive();
        if dead {
            player.player.stand_state = STAND_STATE_DEAD;
        }
        player.last_combat = Some(now);
        MeleeHit {
            attacker,
            victim,
            damage: damage as u32,
            victim_health: player.player.health,
            victim_max_health: player.player.max_health,
            victim_dead: dead,
            lootable: false,
        }
    };
    if attacker != victim {
        mark_combat(inner, attacker, now);
    }
    Some(hit)
}

fn mark_combat(inner: &mut Inner, guid: u64, now: Instant) {
    if let Some(presence) = inner.players.get_mut(&guid) {
        presence.last_combat = Some(now);
    }
}

fn end_combat_for(inner: &mut Inner, guid: u64, events: &mut Vec<WorldEvent>) {
    let mut stops = Vec::new();
    for presence in inner.players.values_mut() {
        if presence.attacking == Some(guid) {
            presence.attacking = None;
            stops.push(Attack {
                attacker: presence.player.guid,
                victim: guid,
                victim_dead: true,
            });
        }
    }
    for state in inner.creatures.values_mut() {
        if state.combat_target == Some(guid) {
            state.combat_target = None;
            stops.push(Attack {
                attacker: state.creature.guid,
                victim: guid,
                victim_dead: true,
            });
        }
        if state.creature.guid == guid {
            if let Some(target) = state.combat_target.take() {
                stops.push(Attack {
                    attacker: guid,
                    victim: target,
                    victim_dead: true,
                });
            }
        }
    }
    events.extend(stops.into_iter().map(WorldEvent::AttackStopped));
}
