use super::*;
use crate::creature::{northshire_guard_guid, northshire_wolf, northshire_wolf_guid};
use crate::gameobject::{northshire_chest, northshire_chest_guid};
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
    assert!(
        nearby
            .iter()
            .any(|creature| creature.guid == northshire_guard_guid())
    );
    assert!(
        nearby
            .iter()
            .any(|creature| creature.guid == northshire_wolf_guid())
    );
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
fn join_at_northshire_sees_a_nearby_gameobject() {
    let world = World::with_gameobjects(vec![northshire_chest()]);
    let nearby = world.gameobjects_near(Position::NORTHSHIRE);
    assert_eq!(nearby.len(), 1);
    assert_eq!(nearby[0].guid, northshire_chest_guid());
}

#[test]
fn visibility_emits_gameobject_when_the_cell_changes() {
    let world = World::with_gameobjects(vec![northshire_chest()]);
    let (mailbox, mut rx) = PlayerMailbox::channel();
    world.join(player(1), mailbox.clone());

    world.broadcast_move(&mailbox, dummy_move(1, Position::NORTHSHIRE));
    let mut appeared = 0;
    while let Ok(event) = rx.try_recv() {
        match event {
            WorldEvent::CreatureAppeared(_) => {}
            WorldEvent::GameObjectAppeared(object) if object.guid == northshire_chest_guid() => {
                appeared += 1;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(appeared, 1);
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
            WorldEvent::AttackStarted(_)
            | WorldEvent::MeleeHit(_)
            | WorldEvent::AttackStopped(_)
            | WorldEvent::CreatureLeft { .. }
            | WorldEvent::CreatureAppeared(_)
            | WorldEvent::LootOpened { .. } => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(moved);
}

#[test]
fn empty_corpse_is_dead_but_not_lootable() {
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
    let corpse = world.creature(northshire_wolf_guid()).unwrap();
    assert!(corpse.dead);
    assert!(!corpse.lootable);

    let mut opened = false;
    while let Ok(event) = rx.try_recv() {
        if matches!(
            event,
            WorldEvent::LootOpened { guid, .. } if guid == northshire_wolf_guid()
        ) {
            opened = true;
        }
    }
    assert!(!opened, "empty roll must not sparkle or open loot");

    world.start_attack(&mailbox, 1, northshire_wolf_guid());
    while let Ok(event) = rx.try_recv() {
        if matches!(event, WorldEvent::LootOpened { .. } | WorldEvent::AttackStarted(_)) {
            panic!("empty corpse should not loot or re-enter combat, got {event:?}");
        }
    }
}

#[test]
fn killing_a_wolf_with_loot_opens_the_window() {
    let mut catalog = crate::catalog::Catalog::empty();
    catalog.loot.insert(
        1,
        vec![crate::catalog::LootRow {
            item_or_ref: 117,
            chance: 100.0,
            group_id: 0,
            min_count: 1,
            max_count: 1,
            condition_id: 0,
        }],
    );
    let mut wolf = northshire_wolf();
    wolf.loot_id = 1;
    let guid = wolf.guid;
    let world = World::with_creatures(vec![wolf], Some(std::sync::Arc::new(catalog)));
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = northshire_wolf().position;
    world.join(hunter, mailbox.clone());
    world.start_attack(&mailbox, 1, guid);
    drain(&mut rx);

    let mut now = Instant::now();
    let mut opened = false;
    for _ in 0..8 {
        world.tick(now);
        now += Duration::from_millis(2000);
        while let Ok(event) = rx.try_recv() {
            if let WorldEvent::LootOpened { guid: loot_guid, items, .. } = event {
                assert_eq!(loot_guid, guid);
                assert_eq!(items[0].item_id, 117);
                opened = true;
            }
        }
        if world.creature(guid).unwrap().dead {
            break;
        }
    }
    assert!(world.creature(guid).unwrap().dead);
    assert!(world.creature(guid).unwrap().lootable);
    assert!(opened, "guaranteed loot should open the window");
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
