use std::sync::Arc;

use wow_shared::{
    CharacterClass, CharacterRace, GossipOptionIcon, GossipOptionKind, MAP_EASTERN_KINGDOMS,
    QuestType,
};

use super::*;
use crate::catalog::{Catalog, QuestRow};
use crate::creature::{
    ENTRY_NORTHSHIRE_GUARD, ENTRY_YOUNG_WOLF, GossipAction, GossipOption, northshire_guard,
    northshire_guard_guid, northshire_wolf, northshire_wolf_guid,
};
use crate::quest::{QUEST_ICON_AVAILABLE, QUEST_ICON_COMPLETE, QuestDialogStatus};

const QUEST_ID: u32 = 9_001;

fn kill_quest() -> QuestRow {
    QuestRow {
        entry: QUEST_ID,
        title: "Young Wolves".into(),
        details: "Kill a young wolf in the courtyard.".into(),
        objectives: "Slay 1 Young Wolf".into(),
        offer_reward_text: "Well done.".into(),
        request_items_text: String::new(),
        end_text: String::new(),
        objective_texts: [
            "Young Wolf slain".into(),
            String::new(),
            String::new(),
            String::new(),
        ],
        min_level: 0,
        quest_level: 1,
        zone_or_sort: 9,
        quest_type: QuestType::None,
        required_races: Vec::new(),
        required_classes: Vec::new(),
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        next_quest_in_chain: 0,
        method: 2,
        quest_flags: 0,
        special_flags: 0,
        src_item_id: 0,
        req_item_id: [0; 4],
        req_item_count: [0; 4],
        req_creature_or_go_id: [ENTRY_YOUNG_WOLF as i32, 0, 0, 0],
        req_creature_or_go_count: [1, 0, 0, 0],
        rew_money: 50,
        rew_money_max_level: 0,
        rew_item_id: [0; 4],
        rew_item_count: [0; 4],
        rew_choice_item_id: [0; 6],
        rew_choice_item_count: [0; 6],
        rew_spell: 0,
        rew_spell_cast: 0,
        point_map_id: 0,
        point_x: 0.0,
        point_y: 0.0,
    }
}

fn quest_world() -> (World, PlayerMailbox, mpsc::UnboundedReceiver<WorldEvent>) {
    let mut catalog = Catalog::empty();
    catalog.insert_quest(kill_quest());
    catalog.link_quest_start(ENTRY_NORTHSHIRE_GUARD, QUEST_ID);
    catalog.link_quest_end(ENTRY_NORTHSHIRE_GUARD, QUEST_ID);
    let world = World::with_catalog(MAP_EASTERN_KINGDOMS, Arc::new(catalog));
    let (mailbox, rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.position = crate::creature::northshire_guard().position;
    world.join(visitor, mailbox.clone());
    (world, mailbox, rx)
}

fn next_event(rx: &mut mpsc::UnboundedReceiver<WorldEvent>) -> WorldEvent {
    rx.try_recv().expect("world event")
}

#[test]
fn questgiver_status_shows_available() {
    let (world, mailbox, mut rx) = quest_world();

    world.questgiver_status(&mailbox, 1, northshire_guard_guid());

    match next_event(&mut rx) {
        WorldEvent::QuestGiverStatus { npc, status } => {
            assert_eq!(npc, northshire_guard_guid());
            assert_eq!(status, QuestDialogStatus::Available.as_protocol());
        }
        other => panic!("expected status, got {other:?}"),
    }
}

#[test]
fn gossip_available_icon_is_dialog_status_available() {
    assert_eq!(
        QUEST_ICON_AVAILABLE,
        QuestDialogStatus::Available.as_protocol()
    );
    assert_eq!(QUEST_ICON_COMPLETE, 4);
}

#[test]
fn complete_without_quest_log_opens_details() {
    let (world, mailbox, mut rx) = quest_world();

    world.complete_quest(&mailbox, 1, northshire_guard_guid(), QUEST_ID);
    match next_event(&mut rx) {
        WorldEvent::QuestDetails { quest, .. } => assert_eq!(quest.entry, QUEST_ID),
        other => panic!("expected details, got {other:?}"),
    }
}

#[test]
fn gossip_lists_available_quest() {
    let (world, mailbox, mut rx) = quest_world();

    world.open_gossip(&mailbox, 1, northshire_guard_guid());

    match next_event(&mut rx) {
        WorldEvent::GossipOpened { quests, .. } => {
            assert_eq!(quests.len(), 1);
            assert_eq!(quests[0].quest_id, QUEST_ID);
            assert_eq!(quests[0].icon, QUEST_ICON_AVAILABLE);
            assert_eq!(quests[0].title, "Young Wolves");
        }
        other => panic!("expected gossip, got {other:?}"),
    }
}

#[test]
fn accept_kill_and_turn_in() {
    let (world, mailbox, mut rx) = quest_world();

    world.query_quest_details(&mailbox, 1, northshire_guard_guid(), QUEST_ID);
    match next_event(&mut rx) {
        WorldEvent::QuestDetails { npc, quest } => {
            assert_eq!(npc, northshire_guard_guid());
            assert_eq!(quest.entry, QUEST_ID);
        }
        other => panic!("expected details, got {other:?}"),
    }

    world.accept_quest(&mailbox, 1, northshire_guard_guid(), QUEST_ID);
    drain_until_log(&mut rx);
    let player_state = world.player(1).expect("player");
    assert_eq!(player_state.quest_log.len(), 1);
    assert_eq!(player_state.quest_log[0].quest_id, QUEST_ID);
    assert!(!player_state.quest_log[0].complete);

    let mut hunter = world.player(1).expect("player");
    hunter.position = northshire_wolf().position;
    world.relocate(1, hunter.position);
    world.start_attack(&mailbox, 1, northshire_wolf_guid());
    drain(&mut rx);

    let mut now = Instant::now();
    let mut credited = false;
    for _ in 0..8 {
        world.tick(now);
        now += Duration::from_secs(2);
        while let Ok(event) = rx.try_recv() {
            if let WorldEvent::QuestKillCredit { credit, .. } = event {
                assert_eq!(credit.quest_id, QUEST_ID);
                assert_eq!(credit.creature_id, ENTRY_YOUNG_WOLF);
                assert_eq!(credit.kill_count, 1);
                credited = true;
            }
        }
        if credited {
            break;
        }
    }
    assert!(credited, "expected kill credit");

    let player_state = world.player(1).expect("player");
    assert!(player_state.quest_log[0].complete);

    world.relocate(1, crate::creature::northshire_guard().position);
    drain(&mut rx);
    world.questgiver_status(&mailbox, 1, northshire_guard_guid());
    match next_event(&mut rx) {
        WorldEvent::QuestGiverStatus { status, .. } => {
            assert_eq!(status, QuestDialogStatus::Reward.as_protocol());
        }
        other => panic!("expected reward status, got {other:?}"),
    }

    world.open_gossip(&mailbox, 1, northshire_guard_guid());
    match next_event(&mut rx) {
        WorldEvent::GossipOpened { quests, .. } => {
            assert_eq!(quests[0].icon, QUEST_ICON_COMPLETE);
        }
        other => panic!("expected complete gossip, got {other:?}"),
    }

    world.complete_quest(&mailbox, 1, northshire_guard_guid(), QUEST_ID);
    match next_event(&mut rx) {
        WorldEvent::QuestOfferReward { quest, .. } => assert_eq!(quest.entry, QUEST_ID),
        other => panic!("expected offer, got {other:?}"),
    }

    let copper_before = world.player(1).expect("player").copper;
    world.choose_quest_reward(&mailbox, 1, northshire_guard_guid(), QUEST_ID, 0);
    let mut turned = false;
    while let Ok(event) = rx.try_recv() {
        if let WorldEvent::QuestTurnedIn {
            quest_id, copper, ..
        } = event
        {
            assert_eq!(quest_id, QUEST_ID);
            assert_eq!(copper, 50);
            turned = true;
        }
    }
    assert!(turned);
    let player_state = world.player(1).expect("player");
    assert!(player_state.quest_log.is_empty());
    assert!(player_state.rewarded_quests.contains(&QUEST_ID));
    assert_eq!(player_state.copper, copper_before + 50);
}

#[test]
fn race_filter_hides_quest() {
    let mut catalog = Catalog::empty();
    let mut quest = kill_quest();
    quest.required_races = vec![CharacterRace::Orc];
    catalog.insert_quest(quest);
    catalog.link_quest_start(ENTRY_NORTHSHIRE_GUARD, QUEST_ID);
    let world = World::with_catalog(MAP_EASTERN_KINGDOMS, Arc::new(catalog));
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.race = CharacterRace::Human;
    visitor.class = CharacterClass::Warrior;
    visitor.position = crate::creature::northshire_guard().position;
    world.join(visitor, mailbox.clone());

    world.questgiver_status(&mailbox, 1, northshire_guard_guid());
    match next_event(&mut rx) {
        WorldEvent::QuestGiverStatus { status, .. } => {
            assert_eq!(status, QuestDialogStatus::None.as_protocol());
        }
        other => panic!("expected none, got {other:?}"),
    }
}

#[test]
fn query_quest_from_gossip_opens_details() {
    let (world, mailbox, mut rx) = quest_world();

    world.open_gossip(&mailbox, 1, northshire_guard_guid());
    drain(&mut rx);

    world.query_quest_details(&mailbox, 1, northshire_guard_guid(), QUEST_ID);
    match next_event(&mut rx) {
        WorldEvent::QuestDetails { npc, quest } => {
            assert_eq!(npc, northshire_guard_guid());
            assert_eq!(quest.entry, QUEST_ID);
        }
        other => panic!("expected details, got {other:?}"),
    }
}

#[test]
fn gossip_select_quest_id_opens_details() {
    let (world, mailbox, mut rx) = quest_world();

    world.open_gossip(&mailbox, 1, northshire_guard_guid());
    drain(&mut rx);

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), QUEST_ID);
    match next_event(&mut rx) {
        WorldEvent::QuestDetails { quest, .. } => assert_eq!(quest.entry, QUEST_ID),
        other => panic!("expected details, got {other:?}"),
    }
}

#[test]
fn questgiver_gossip_option_opens_details() {
    let mut catalog = Catalog::empty();
    catalog.insert_quest(kill_quest());
    catalog.link_quest_start(ENTRY_NORTHSHIRE_GUARD, QUEST_ID);
    let mut guard = northshire_guard();
    guard.gossip.as_mut().unwrap().menus[0]
        .options
        .push(GossipOption {
            id: 50,
            text: "Let me see your quests.".into(),
            icon: GossipOptionIcon::Chat,
            kind: GossipOptionKind::QuestGiver,
            action: GossipAction::OpenQuestGiver,
        });
    let world = World::with_creatures(vec![guard, northshire_wolf()], Some(Arc::new(catalog)));
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.position = crate::creature::northshire_guard().position;
    world.join(visitor, mailbox.clone());

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), 50);
    match next_event(&mut rx) {
        WorldEvent::QuestDetails { quest, .. } => assert_eq!(quest.entry, QUEST_ID),
        other => panic!("expected details, got {other:?}"),
    }
}

#[test]
fn questgiver_gossip_option_opens_list_when_several() {
    let mut catalog = Catalog::empty();
    let mut second = kill_quest();
    second.entry = 9_002;
    second.title = "More Wolves".into();
    catalog.insert_quest(kill_quest());
    catalog.insert_quest(second);
    catalog.link_quest_start(ENTRY_NORTHSHIRE_GUARD, QUEST_ID);
    catalog.link_quest_start(ENTRY_NORTHSHIRE_GUARD, 9_002);
    let mut guard = northshire_guard();
    guard.gossip.as_mut().unwrap().menus[0]
        .options
        .push(GossipOption {
            id: 50,
            text: "Let me see your quests.".into(),
            icon: GossipOptionIcon::Chat,
            kind: GossipOptionKind::QuestGiver,
            action: GossipAction::OpenQuestGiver,
        });
    let world = World::with_creatures(vec![guard, northshire_wolf()], Some(Arc::new(catalog)));
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.position = crate::creature::northshire_guard().position;
    world.join(visitor, mailbox.clone());

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), 50);
    match next_event(&mut rx) {
        WorldEvent::GossipClosed => {}
        other => panic!("expected gossip close, got {other:?}"),
    }
    match next_event(&mut rx) {
        WorldEvent::QuestList { quests, .. } => {
            assert_eq!(quests.len(), 2);
        }
        other => panic!("expected quest list, got {other:?}"),
    }
}

fn drain_until_log(rx: &mut mpsc::UnboundedReceiver<WorldEvent>) {
    let mut seen = false;
    while let Ok(event) = rx.try_recv() {
        if matches!(event, WorldEvent::QuestLogUpdate { .. }) {
            seen = true;
        }
    }
    assert!(seen, "expected quest log update");
}
