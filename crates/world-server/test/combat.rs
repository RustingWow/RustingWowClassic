use super::*;

#[test]
fn northshire_has_a_guard_and_a_wolf() {
    let world = World::new();
    let creatures = world.creatures();
    assert!(
        creatures
            .iter()
            .any(|c| c.guid == northshire_guard_guid() && !c.hostile)
    );
    assert!(
        creatures
            .iter()
            .any(|c| c.guid == northshire_wolf_guid() && c.hostile)
    );
}

#[test]
fn friendly_npc_cannot_be_attacked() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    world.join(player(1), mailbox.clone());

    world.start_attack(&mailbox, 1, northshire_guard_guid());

    match rx.try_recv().expect("reject attack") {
        WorldEvent::AttackStopped(attack) => {
            assert_eq!(attack.attacker, 1);
            assert_eq!(attack.victim, northshire_guard_guid());
        }
        other => panic!("expected attack stop, got {other:?}"),
    }
    assert_eq!(
        world.creature(northshire_guard_guid()).unwrap().health,
        world.creature(northshire_guard_guid()).unwrap().max_health
    );
}

#[test]
fn wolf_does_not_aggro_at_spawn_distance() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut spawn = player(1);
    spawn.position = Position::NORTHSHIRE;
    world.join(spawn, mailbox);
    world.tick(Instant::now());
    assert!(rx.try_recv().is_err());
    assert!(world.creature(northshire_wolf_guid()).unwrap().health > 0);
}

#[test]
fn attacking_the_wolf_deals_damage_in_melee() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = northshire_wolf().position;
    world.join(hunter, mailbox.clone());

    world.start_attack(&mailbox, 1, northshire_wolf_guid());
    drain(&mut rx);

    world.tick(Instant::now());
    let hit = loop {
        match rx.try_recv().expect("combat event") {
            WorldEvent::MeleeHit(hit) if hit.attacker == 1 => break hit,
            WorldEvent::MeleeHit(_) | WorldEvent::AttackStarted(_) => continue,
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(hit.victim, northshire_wolf_guid());
    assert_eq!(hit.damage, PLAYER_DAMAGE as u32);
    assert_eq!(
        world.creature(northshire_wolf_guid()).unwrap().health,
        northshire_wolf().max_health - PLAYER_DAMAGE
    );
}

#[test]
fn wolf_aggroes_when_a_player_walks_close() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = northshire_wolf().position;
    world.join(hunter, mailbox);
    world.tick(Instant::now());

    let started = loop {
        match rx.try_recv().expect("aggro") {
            WorldEvent::AttackStarted(attack) if attack.attacker == northshire_wolf_guid() => {
                break attack;
            }
            WorldEvent::MeleeHit(_) | WorldEvent::AttackStarted(_) => continue,
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(started.victim, 1);
}
