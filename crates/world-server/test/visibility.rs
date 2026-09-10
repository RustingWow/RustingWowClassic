use super::*;
use crate::creature::{northshire_guard_guid, northshire_wolf, northshire_wolf_guid};
use crate::quest_director::TaskObjective;
use std::time::{Duration, Instant};

#[test]
fn join_at_origin_does_not_show_northshire_npcs() {
    let world = World::new();
    let origin = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        orientation: 0.0,
    };
    assert!(world.creatures_near(origin).is_empty());
    let (mailbox, mut rx) = PlayerMailbox::channel();
    world.join(player(1), mailbox);
    assert!(rx.try_recv().is_err());
}

#[test]
fn join_at_northshire_sees_the_local_npcs() {
    let world = World::new();
    let nearby = world.creatures_near(Position::NORTHSHIRE);
    assert_eq!(nearby.len(), 2);
    assert!(nearby.iter().any(|creature| creature.guid == northshire_guard_guid()));
    assert!(nearby.iter().any(|creature| creature.guid == northshire_wolf_guid()));
}

#[test]
fn visibility_updates_only_when_the_cell_changes() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    world.join(player(1), mailbox.clone());

    world.broadcast_move(
        &mailbox,
        dummy_move(
            1,
            Position {
                x: 5.0,
                y: 0.0,
                z: 0.0,
                orientation: 0.0,
            },
        ),
    );
    assert!(rx.try_recv().is_err());

    world.broadcast_move(&mailbox, dummy_move(1, Position::NORTHSHIRE));
    let mut appeared = 0;
    while let Ok(event) = rx.try_recv() {
        match event {
            WorldEvent::CreatureAppeared(_) => appeared += 1,
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(appeared, 2);
}

#[test]
fn wolf_chases_when_the_player_is_out_of_melee() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = Position {
        x: northshire_wolf().position.x + 10.0,
        y: northshire_wolf().position.y,
        z: northshire_wolf().position.z,
        orientation: 0.0,
    };
    world.join(hunter, mailbox);
    world.tick(Instant::now());

    let mut moved = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            WorldEvent::CreatureMoved { guid, .. } if guid == northshire_wolf_guid() => {
                moved = true;
            }
            WorldEvent::AttackStarted(_) | WorldEvent::MeleeHit(_) => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(moved);
}

#[test]
fn looting_a_dead_wolf_opens_an_empty_window_without_catalog() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = northshire_wolf().position;
    world.join(hunter, mailbox.clone());
    world.start_attack(&mailbox, 1, northshire_wolf_guid());
    drain(&mut rx);

    let mut now = Instant::now();
    for _ in 0..8 {
        world.tick(now);
        now += Duration::from_millis(2000);
        if world.creature(northshire_wolf_guid()).unwrap().dead {
            break;
        }
    }
    assert!(world.creature(northshire_wolf_guid()).unwrap().dead);

    drain(&mut rx);
    world.open_loot(&mailbox, 1, northshire_wolf_guid());
    match rx.try_recv().expect("loot") {
        WorldEvent::LootOpened { guid, items, .. } => {
            assert_eq!(guid, northshire_wolf_guid());
            assert!(items.is_empty());
        }
        other => panic!("expected loot, got {other:?}"),
    }
}

#[test]
fn quest_director_picks_a_nearby_hostile_entry() {
    let world = World::new();
    let (mailbox, _rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = Position::NORTHSHIRE;
    world.join(hunter, mailbox);
    let task = world.suggest_task(1).expect("task");
    match task.objective {
        TaskObjective::Kill { entry, count } => {
            assert_eq!(entry, crate::creature::ENTRY_YOUNG_WOLF);
            assert_eq!(count, 4);
        }
        other => panic!("expected kill, got {other:?}"),
    }
    assert!(task.flavor.is_empty());
}
