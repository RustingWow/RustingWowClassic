use std::collections::{HashMap, HashSet};

use wow_shared::Position;

use crate::creature::Creature;
use crate::gameobject::{GameObject, is_gameobject_guid};

use super::{
    CELL_HYSTERESIS, CELL_SIZE, CREATE_RANGE, DESTROY_RANGE, Inner, PlayerMailbox, World,
    WorldEvent, broadcast,
};

impl World {
    pub fn creatures_near(&self, position: Position) -> Vec<Creature> {
        let inner = self.inner.lock().expect("world mutex");
        creatures_in_range(&inner, position, CREATE_RANGE)
    }

    pub fn gameobjects_near(&self, position: Position) -> Vec<GameObject> {
        let inner = self.inner.lock().expect("world mutex");
        gameobjects_in_range(&inner, position, CREATE_RANGE)
    }

    pub fn broadcast_move(&self, from: &PlayerMailbox, movement: super::Movement) {
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

    pub fn relocate(&self, guid: u64, position: Position) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&guid) else {
            return;
        };
        presence.player.position = position;
        let mailbox = presence.mailbox.clone();
        refresh_visibility(&mut inner, guid, &mailbox);
    }

    pub fn spawn_temp_npc(&self, entry: u32, position: Position) -> Option<Creature> {
        let catalog = self.catalog.as_ref()?;
        let mut inner = self.inner.lock().expect("world mutex");
        inner.temp_counter = inner.temp_counter.wrapping_add(1).max(0xF000_0001);
        let guid_counter = inner.temp_counter;
        let creature = catalog.spawn_creature(entry, guid_counter, position)?;
        inner
            .grid
            .entry(creature.position.cell(CELL_SIZE))
            .or_default()
            .insert(creature.guid);
        inner.creatures.insert(
            creature.guid,
            super::CreatureState {
                home: creature.position,
                creature: creature.clone(),
                combat_target: None,
                next_swing: std::time::Instant::now(),
                died_at: None,
                damage: creature.melee_damage.max(0),
            },
        );
        dispatch_event(&mut inner, WorldEvent::CreatureAppeared(creature.clone()));
        Some(creature)
    }

    pub fn delete_npc(&self, guid: u64) -> bool {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(state) = inner.creatures.remove(&guid) else {
            return false;
        };
        let cell = state.creature.position.cell(CELL_SIZE);
        if let Some(bucket) = inner.grid.get_mut(&cell) {
            bucket.remove(&guid);
            if bucket.is_empty() {
                inner.grid.remove(&cell);
            }
        }
        dispatch_event(&mut inner, WorldEvent::CreatureLeft { guid });
        true
    }
}

pub(crate) fn dispatch_event(inner: &mut Inner, event: WorldEvent) {
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
        WorldEvent::GameObjectAppeared(object) => {
            for presence in inner.players.values_mut() {
                if in_range(presence.player.position, object.position, CREATE_RANGE) {
                    presence.visible.insert(object.guid);
                    presence.mailbox.send(event.clone());
                }
            }
        }
        WorldEvent::GameObjectLeft { guid } => {
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
        WorldEvent::LootOpened { guid, .. } => {
            for presence in inner.players.values() {
                if presence.looting == Some(*guid) {
                    presence.mailbox.send(event.clone());
                }
            }
        }
        _ => broadcast(inner, event),
    }
}

pub(crate) fn creatures_in_range(inner: &Inner, origin: Position, range: f32) -> Vec<Creature> {
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

pub(crate) fn gameobjects_in_range(inner: &Inner, origin: Position, range: f32) -> Vec<GameObject> {
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
                let Some(object) = inner.gameobjects.get(guid) else {
                    continue;
                };
                if origin.distance_squared(object.position) <= range_sq {
                    out.push(object.clone());
                }
            }
        }
    }
    out
}

pub(crate) fn refresh_visibility(inner: &mut Inner, player: u64, mailbox: &PlayerMailbox) {
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
        .chain(
            gameobjects_in_range(inner, position, CREATE_RANGE)
                .into_iter()
                .map(|object| object.guid),
        )
        .collect();
    let destroy_sq = DESTROY_RANGE * DESTROY_RANGE;
    let mut next = HashSet::new();
    for guid in current.iter().copied() {
        if let Some(state) = inner.creatures.get(&guid) {
            if position.distance_squared(state.creature.position) <= destroy_sq {
                next.insert(guid);
            } else {
                mailbox.send(WorldEvent::CreatureLeft { guid });
            }
            continue;
        }
        if let Some(object) = inner.gameobjects.get(&guid) {
            if position.distance_squared(object.position) <= destroy_sq {
                next.insert(guid);
            } else {
                mailbox.send(WorldEvent::GameObjectLeft { guid });
            }
            continue;
        }
        mailbox.send(if is_gameobject_guid(guid) {
            WorldEvent::GameObjectLeft { guid }
        } else {
            WorldEvent::CreatureLeft { guid }
        });
    }
    for guid in nearby {
        if !next.insert(guid) {
            continue;
        }
        if let Some(state) = inner.creatures.get(&guid) {
            mailbox.send(WorldEvent::CreatureAppeared(state.creature.clone()));
        } else if let Some(object) = inner.gameobjects.get(&guid) {
            mailbox.send(WorldEvent::GameObjectAppeared(object.clone()));
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

pub(crate) fn move_grid(
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

pub(crate) fn sync_creature_viewers(inner: &mut Inner, guid: u64) {
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

pub(crate) fn in_range(a: Position, b: Position, range: f32) -> bool {
    a.distance_squared(b) <= range * range
}
