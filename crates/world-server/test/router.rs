use super::*;
use crate::directory::Directory;
use crate::player::Player;
use crate::world::{Chat, ChatChannel, PlayerMailbox, WorldEvent};
use wow_shared::{MAP_EASTERN_KINGDOMS, MAP_KALIMDOR, Position};

fn player_on(guid: u64, position: Position) -> Player {
    Player::new(guid, format!("User{guid}"), position)
}

#[test]
fn transfer_moves_player_between_in_process_maps() {
    let router = MapRouter::from_shards(&wow_shared::ShardFile::builtin(), Directory::memory());
    let (mailbox, _rx) = PlayerMailbox::channel();
    let origin = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        orientation: 0.0,
    };
    router
        .join(MAP_EASTERN_KINGDOMS, player_on(1, origin), mailbox.clone())
        .expect("join ek");
    assert!(
        router
            .map(MAP_EASTERN_KINGDOMS)
            .unwrap()
            .player(1)
            .is_some()
    );

    let dest = Position {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        orientation: 1.0,
    };
    router
        .transfer(
            MAP_EASTERN_KINGDOMS,
            MAP_KALIMDOR,
            player_on(1, dest),
            mailbox,
        )
        .expect("transfer");

    assert!(
        router
            .map(MAP_EASTERN_KINGDOMS)
            .unwrap()
            .player(1)
            .is_none()
    );
    let moved = router.map(MAP_KALIMDOR).unwrap().player(1).unwrap();
    assert_eq!(moved.position, dest);
    assert_eq!(router.directory().map_of(1), Some(MAP_KALIMDOR));
}

#[test]
fn whisper_reaches_player_on_another_map() {
    let router = MapRouter::from_shards(&wow_shared::ShardFile::builtin(), Directory::memory());
    let origin = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        orientation: 0.0,
    };
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    router
        .join(MAP_EASTERN_KINGDOMS, player_on(1, origin), mailbox1)
        .unwrap();
    let (mailbox2, mut rx2) = PlayerMailbox::channel();
    router
        .join(MAP_KALIMDOR, player_on(2, origin), mailbox2.clone())
        .unwrap();

    router.speak(
        MAP_KALIMDOR,
        &mailbox2,
        Chat {
            speaker: 2,
            channel: ChatChannel::Whisper {
                to: "user1".to_string(),
            },
            text: "across".to_string(),
        },
    );

    match rx1.try_recv().expect("whisper") {
        WorldEvent::Chat(spoken) => {
            assert_eq!(spoken.from_guid, 2);
            assert_eq!(spoken.text, "across");
        }
        other => panic!("expected chat, got {other:?}"),
    }
    match rx2.try_recv().expect("inform") {
        WorldEvent::Chat(spoken) => {
            assert_eq!(spoken.from_guid, 1);
            assert_eq!(spoken.delivery, crate::world::ChatDelivery::WhisperInform);
        }
        other => panic!("expected inform, got {other:?}"),
    }
}
