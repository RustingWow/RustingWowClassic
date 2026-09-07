use super::*;
use crate::player::Player;
use crate::world::{ChatDelivery, PlayerMailbox, WorldEvent};
use wow_shared::{MAP_EASTERN_KINGDOMS, MAP_KALIMDOR, Position};

#[test]
fn who_lists_registered_players() {
    let directory = Directory::memory();
    let (mailbox, _rx) = PlayerMailbox::channel();
    directory.register(
        1,
        "User1".to_string(),
        MAP_EASTERN_KINGDOMS,
        mailbox.clone(),
    );
    directory.register(2, "User2".to_string(), MAP_KALIMDOR, mailbox);
    let mut who = directory.who();
    who.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        who,
        vec![
            ("User1".to_string(), MAP_EASTERN_KINGDOMS),
            ("User2".to_string(), MAP_KALIMDOR),
        ]
    );
}

#[test]
fn whisper_unknown_name_notifies_sender() {
    let directory = Directory::memory();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    directory.register(
        1,
        "User1".to_string(),
        MAP_EASTERN_KINGDOMS,
        mailbox.clone(),
    );
    directory.whisper(&mailbox, 1, "Nobody".to_string(), "hey".to_string());
    match rx.try_recv().expect("not found") {
        WorldEvent::ChatPlayerNotFound { name } => assert_eq!(name, "Nobody"),
        other => panic!("expected not found, got {other:?}"),
    }
}

#[test]
fn name_query_survives_map_change() {
    let directory = Directory::memory();
    let (mailbox, _rx) = PlayerMailbox::channel();
    directory.register(
        1,
        "User1".to_string(),
        MAP_EASTERN_KINGDOMS,
        mailbox.clone(),
    );
    directory.set_map(1, MAP_KALIMDOR);
    assert_eq!(directory.name(1).as_deref(), Some("User1"));
    assert_eq!(directory.map_of(1), Some(MAP_KALIMDOR));
    let _player = Player::new(
        1,
        "User1",
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
        },
    );
    let _ = ChatDelivery::Whisper;
}
