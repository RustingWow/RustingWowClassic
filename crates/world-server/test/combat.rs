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
            WorldEvent::CreatureLeft { .. }
            | WorldEvent::CreatureAppeared(_)
            | WorldEvent::LootOpened { .. }
            | WorldEvent::AttackStopped(_) => continue,
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
fn gm_mode_ignores_incoming_damage() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = northshire_wolf().position;
    hunter.gm_on = true;
    world.join(hunter, mailbox);
    world.tick(Instant::now());
    drain(&mut rx);
    world.tick(Instant::now() + Duration::from_secs(2));
    assert_eq!(world.player(1).unwrap().health, PLAYER_MAX_HEALTH);
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
            WorldEvent::CreatureLeft { .. }
            | WorldEvent::CreatureAppeared(_)
            | WorldEvent::LootOpened { .. }
            | WorldEvent::AttackStopped(_) => continue,
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(started.victim, 1);
}

#[test]
fn demo_wolf_keeps_monster_faction() {
    assert_eq!(northshire_wolf().faction, FACTION_MONSTER);
    assert!(northshire_wolf().hostile);
}

#[test]
fn demo_guard_stays_stormwind_and_unattackable() {
    assert_eq!(northshire_guard().faction, FACTION_STORMWIND);
    assert!(!northshire_guard().hostile);
}

#[test]
fn catalog_wolf_with_template_32_can_be_attacked() {
    let mut catalog = crate::catalog::Catalog::empty();
    catalog.types.insert(
        299,
        crate::catalog::CreatureTypeRow {
            entry: 299,
            name: "Young Wolf".into(),
            sub_name: String::new(),
            level: 1,
            max_level: 1,
            display_id: 447,
            faction: 32,
            family: 1,
            creature_type: 1,
            npc_flags: 0,
            unit_flags: 0,
            civilian: false,
            health: 45,
            melee_damage: 4,
            loot_id: 0,
            gossip_menu_id: 0,
            vendor_template_id: 0,
            skinning_loot_id: 0,
            pickpocket_loot_id: 0,
        },
    );
    let wolf = catalog
        .spawn_creature(299, 1, Position::NORTHSHIRE)
        .expect("spawn");
    assert_eq!(wolf.faction, 38);
    assert!(wolf.hostile);

    let world = World::with_creatures(vec![wolf.clone()], None);
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.position = wolf.position;
    world.join(hunter, mailbox.clone());
    drain(&mut rx);

    world.start_attack(&mailbox, 1, wolf.guid);
    match rx.try_recv().expect("attack") {
        WorldEvent::AttackStarted(attack) => {
            assert_eq!(attack.attacker, 1);
            assert_eq!(attack.victim, wolf.guid);
        }
        other => panic!("expected attack start, got {other:?}"),
    }
}

#[test]
fn health_regen_is_five_percent_standing_and_double_sitting() {
    assert_eq!(
        super::super::combat::health_regen_amount(PLAYER_MAX_HEALTH, 0),
        5
    );
    assert_eq!(
        super::super::combat::health_regen_amount(PLAYER_MAX_HEALTH, STAND_STATE_SIT),
        10
    );
}

#[test]
fn injured_player_regenerates_out_of_combat() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut wounded = player(1);
    wounded.health = 50;
    world.join(wounded, mailbox);
    drain(&mut rx);

    let now = Instant::now();
    world.tick(now);
    assert_eq!(world.player(1).unwrap().health, 50);

    world.tick(now + Duration::from_secs(2));
    assert_eq!(world.player(1).unwrap().health, 55);
}

#[test]
fn sitting_player_regenerates_faster() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut wounded = player(1);
    wounded.health = 50;
    world.join(wounded, mailbox.clone());
    drain(&mut rx);
    world.change_stand_state(&mailbox, 1, STAND_STATE_SIT);
    drain(&mut rx);

    world.tick(Instant::now() + Duration::from_secs(2));
    assert_eq!(world.player(1).unwrap().health, 60);
}

#[test]
fn attacking_player_does_not_regenerate() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut hunter = player(1);
    hunter.health = 50;
    hunter.position = northshire_wolf().position;
    world.join(hunter, mailbox.clone());
    world.start_attack(&mailbox, 1, northshire_wolf_guid());
    drain(&mut rx);

    world.tick(Instant::now() + Duration::from_secs(2));
    assert!(world.player(1).unwrap().health <= 50);
}
