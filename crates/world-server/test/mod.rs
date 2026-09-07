#![allow(dead_code)]

use super::*;
use crate::creature::{northshire_guard_guid, northshire_wolf, northshire_wolf_guid};
use crate::player::PLAYER_DAMAGE;
use wow_world_messages::vanilla::opcodes::ServerOpcodeMessage;

fn player(guid: u64) -> Player {
    Player::new(
        guid,
        format!("User{guid}"),
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
        },
    )
}

fn dummy_move(guid: u64, position: Position) -> Movement {
    Movement::new(guid, position, ServerOpcodeMessage::SMSG_LOGOUT_COMPLETE)
}

fn drain(rx: &mut mpsc::UnboundedReceiver<WorldEvent>) {
    while rx.try_recv().is_ok() {}
}

mod chat;
mod combat;
mod gossip;
mod players;
mod stand_state;
