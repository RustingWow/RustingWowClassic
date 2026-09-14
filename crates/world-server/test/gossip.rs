use super::*;
use crate::creature::{
    GUARD_GOSSIP_ACCEPT_ID, GUARD_GOSSIP_ASK_ID, GUARD_GOSSIP_TEXT, GUARD_GOSSIP_TEXT_ID,
    GUARD_GOSSIP_WOLF_TEXT, GUARD_GOSSIP_WOLF_TEXT_ID, GossipAction, GossipOption,
    northshire_guard, northshire_guard_guid, northshire_wolf_guid,
};
use wow_shared::{GossipOptionIcon, GossipOptionKind};

fn talking_to_guard() -> (World, PlayerMailbox, mpsc::UnboundedReceiver<WorldEvent>) {
    let world = World::new();
    let (mailbox, rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.position = northshire_guard().position;
    world.join(visitor, mailbox.clone());
    (world, mailbox, rx)
}

#[test]
fn talking_to_the_guard_opens_gossip() {
    let (world, mailbox, mut rx) = talking_to_guard();

    world.open_gossip(&mailbox, 1, northshire_guard_guid());

    match rx.try_recv().expect("gossip") {
        WorldEvent::GossipOpened { npc, menu, .. } => {
            assert_eq!(npc, northshire_guard_guid());
            assert_eq!(menu.text_id, GUARD_GOSSIP_TEXT_ID);
            assert_eq!(menu.text, GUARD_GOSSIP_TEXT);
            assert_eq!(menu.options.len(), 1);
            assert_eq!(menu.options[0].id, GUARD_GOSSIP_ASK_ID);
            assert_eq!(menu.options[0].text, "What's nearby?");
        }
        other => panic!("expected gossip, got {other:?}"),
    }
}

#[test]
fn asking_the_guard_opens_the_wolf_menu() {
    let (world, mailbox, mut rx) = talking_to_guard();

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), GUARD_GOSSIP_ASK_ID);

    match rx.try_recv().expect("gossip") {
        WorldEvent::GossipOpened { npc, menu, .. } => {
            assert_eq!(npc, northshire_guard_guid());
            assert_eq!(menu.text_id, GUARD_GOSSIP_WOLF_TEXT_ID);
            assert_eq!(menu.text, GUARD_GOSSIP_WOLF_TEXT);
            assert_eq!(menu.options[0].id, GUARD_GOSSIP_ACCEPT_ID);
        }
        other => panic!("expected wolf menu, got {other:?}"),
    }
}

#[test]
fn accepting_the_wolf_task_closes_gossip() {
    let (world, mailbox, mut rx) = talking_to_guard();

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), GUARD_GOSSIP_ACCEPT_ID);

    match rx.try_recv().expect("close") {
        WorldEvent::GossipClosed => {}
        other => panic!("expected close, got {other:?}"),
    }
}

#[test]
fn unknown_gossip_option_is_ignored() {
    let (world, mailbox, mut rx) = talking_to_guard();

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), 99);

    assert!(rx.try_recv().is_err());
}

#[test]
fn wolf_has_no_gossip() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.position = Position::NORTHSHIRE;
    world.join(visitor, mailbox.clone());

    world.open_gossip(&mailbox, 1, northshire_wolf_guid());
    world.select_gossip_option(&mailbox, 1, northshire_wolf_guid(), 0);

    assert!(rx.try_recv().is_err());
}

#[test]
fn gossip_is_ignored_when_too_far_away() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    world.join(player(1), mailbox.clone());

    world.open_gossip(&mailbox, 1, northshire_guard_guid());
    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), GUARD_GOSSIP_ASK_ID);

    assert!(rx.try_recv().is_err());
}

#[test]
fn npc_text_query_returns_guard_pages() {
    let world = World::new();
    assert_eq!(
        world.npc_text(GUARD_GOSSIP_TEXT_ID).as_deref(),
        Some(GUARD_GOSSIP_TEXT)
    );
    assert_eq!(
        world.npc_text(GUARD_GOSSIP_WOLF_TEXT_ID).as_deref(),
        Some(GUARD_GOSSIP_WOLF_TEXT)
    );
    assert!(world.npc_text(1).is_none());
}

#[test]
fn unhandled_gossip_kind_closes() {
    let mut guard = northshire_guard();
    guard.gossip.as_mut().unwrap().menus[0]
        .options
        .push(GossipOption {
            id: 7,
            text: "Train me.".into(),
            icon: GossipOptionIcon::Trainer,
            kind: GossipOptionKind::Trainer,
            action: GossipAction::Close,
        });
    let world = World::with_creatures(vec![guard], None);
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut visitor = player(1);
    visitor.position = northshire_guard().position;
    world.join(visitor, mailbox.clone());

    world.select_gossip_option(&mailbox, 1, northshire_guard_guid(), 7);

    match rx.try_recv().expect("close") {
        WorldEvent::GossipClosed => {}
        other => panic!("expected close, got {other:?}"),
    }
}
