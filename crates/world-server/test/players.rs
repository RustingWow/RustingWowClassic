use super::*;

#[test]
fn second_player_sees_existing_and_notifies_first() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    let others = world.join(player(1), mailbox1);
    assert!(others.is_empty());

    let (mailbox2, mut rx2) = PlayerMailbox::channel();
    let others = world.join(player(2), mailbox2);
    assert_eq!(others.len(), 1);
    assert_eq!(others[0].guid, 1);

    match rx1.try_recv().expect("first player should be notified") {
        WorldEvent::PlayerAppeared(appeared) => assert_eq!(appeared.guid, 2),
        other => panic!("expected appear, got {other:?}"),
    }
    assert!(rx2.try_recv().is_err());
}

#[test]
fn leave_hides_player_from_the_one_who_stays() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    world.join(player(1), mailbox1.clone());
    let (mailbox2, _rx2) = PlayerMailbox::channel();
    world.join(player(2), mailbox2.clone());
    let _ = rx1.try_recv();

    world.leave(2, &mailbox2);

    match rx1.try_recv().expect("despawn") {
        WorldEvent::PlayerLeft { guid } => assert_eq!(guid, 2),
        other => panic!("expected leave, got {other:?}"),
    }
    assert!(world.player(2).is_none());
    assert!(world.player(1).is_some());
}

#[test]
fn stale_leave_does_not_remove_replacement_session() {
    let world = World::new();
    let (old_mailbox, _old_rx) = PlayerMailbox::channel();
    world.join(player(1), old_mailbox.clone());
    let (new_mailbox, _new_rx) = PlayerMailbox::channel();
    world.join(player(1), new_mailbox);
    world.leave(1, &old_mailbox);
    assert!(world.player(1).is_some());
}

#[test]
fn movement_is_forwarded_to_others_and_updates_position() {
    let world = World::new();
    let (mailbox1, mut rx1) = PlayerMailbox::channel();
    world.join(player(1), mailbox1);
    let (mailbox2, mut rx2) = PlayerMailbox::channel();
    world.join(player(2), mailbox2.clone());
    let _ = rx1.try_recv();

    let position = Position {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        orientation: 1.5,
    };
    world.broadcast_move(&mailbox2, dummy_move(2, position));

    match rx1.try_recv().expect("movement") {
        WorldEvent::PlayerMoved(moved) => {
            assert_eq!(moved.guid, 2);
            assert_eq!(moved.position, position);
        }
        other => panic!("expected movement, got {other:?}"),
    }
    assert!(rx2.try_recv().is_err());
    assert_eq!(world.player(2).unwrap().position, position);
}
