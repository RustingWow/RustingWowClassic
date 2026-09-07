use super::*;

#[test]
fn sitting_is_acked_and_shown_to_nearby_players() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    world.join(player(1), mailbox1);
    let (mailbox2, mut rx2) = PlayerMailbox::channel();
    world.join(player(2), mailbox2.clone());
    let _ = rx1.try_recv();

    world.change_stand_state(&mailbox2, 2, 1);

    match rx2.try_recv().expect("ack") {
        WorldEvent::StandStateAck { state } => assert_eq!(state, 1),
        other => panic!("expected ack, got {other:?}"),
    }
    match rx2.try_recv().expect("self update") {
        WorldEvent::PlayerStandState { guid, state } => {
            assert_eq!(guid, 2);
            assert_eq!(state, 1);
        }
        other => panic!("expected stand state, got {other:?}"),
    }
    match rx1.try_recv().expect("nearby update") {
        WorldEvent::PlayerStandState { guid, state } => {
            assert_eq!(guid, 2);
            assert_eq!(state, 1);
        }
        other => panic!("expected stand state, got {other:?}"),
    }
    assert_eq!(world.player(2).unwrap().stand_state, 1);
}

#[test]
fn dead_player_cannot_change_stand_state() {
    let world = World::new();
    let (mailbox, mut rx) = PlayerMailbox::channel();
    let mut corpse = player(1);
    corpse.health = 0;
    corpse.stand_state = STAND_STATE_DEAD;
    world.join(corpse, mailbox.clone());

    world.change_stand_state(&mailbox, 1, 1);

    assert!(rx.try_recv().is_err());
    assert_eq!(world.player(1).unwrap().stand_state, STAND_STATE_DEAD);
}
