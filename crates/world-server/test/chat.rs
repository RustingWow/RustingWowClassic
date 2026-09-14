use super::*;

#[test]
fn say_reaches_speaker_and_nearby_player() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    world.join(player(1), mailbox1);
    let (mailbox2, mut rx2) = PlayerMailbox::channel();
    world.join(player(2), mailbox2.clone());
    let _ = rx1.try_recv();

    world.speak(
        &mailbox2,
        Chat {
            speaker: 2,
            channel: ChatChannel::Say,
            text: "hello".to_string(),
            gm_tag: false,
        },
    );

    match rx1.try_recv().expect("hear say") {
        WorldEvent::Chat(spoken) => {
            assert_eq!(spoken.from_guid, 2);
            assert_eq!(spoken.text, "hello");
            assert_eq!(spoken.delivery, ChatDelivery::Say);
        }
        other => panic!("expected chat, got {other:?}"),
    }
    match rx2.try_recv().expect("echo say") {
        WorldEvent::Chat(spoken) => assert_eq!(spoken.from_guid, 2),
        other => panic!("expected chat echo, got {other:?}"),
    }
}

#[test]
fn whisper_reaches_named_player_and_informs_sender() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    world.join(player(1), mailbox1);
    let (mailbox2, mut rx2) = PlayerMailbox::channel();
    world.join(player(2), mailbox2.clone());
    let _ = rx1.try_recv();

    world.speak(
        &mailbox2,
        Chat {
            speaker: 2,
            channel: ChatChannel::Whisper {
                to: "user1".to_string(),
            },
            text: "psst".to_string(),
            gm_tag: false,
        },
    );

    match rx1.try_recv().expect("whisper") {
        WorldEvent::Chat(spoken) => {
            assert_eq!(spoken.from_guid, 2);
            assert_eq!(spoken.delivery, ChatDelivery::Whisper);
            assert_eq!(spoken.text, "psst");
        }
        other => panic!("expected whisper, got {other:?}"),
    }
    match rx2.try_recv().expect("inform") {
        WorldEvent::Chat(spoken) => {
            assert_eq!(spoken.from_guid, 1);
            assert_eq!(spoken.delivery, ChatDelivery::WhisperInform);
        }
        other => panic!("expected whisper inform, got {other:?}"),
    }
}

#[test]
fn whisper_unknown_name_notifies_sender() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    world.join(player(1), mailbox1.clone());

    world.speak(
        &mailbox1,
        Chat {
            speaker: 1,
            channel: ChatChannel::Whisper {
                to: "Nobody".to_string(),
            },
            text: "hey".to_string(),
            gm_tag: false,
        },
    );

    match rx1.try_recv().expect("not found") {
        WorldEvent::ChatPlayerNotFound { name } => assert_eq!(name, "Nobody"),
        other => panic!("expected not found, got {other:?}"),
    }
}
