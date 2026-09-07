use wow_world_messages::Guid;
use wow_world_messages::vanilla::opcodes::{ClientOpcodeMessage, ServerOpcodeMessage};
use wow_world_messages::vanilla::{
    MSG_MOVE_FALL_LAND_Server, MSG_MOVE_HEARTBEAT_Server, MSG_MOVE_JUMP_Server,
    MSG_MOVE_SET_FACING_Server, MSG_MOVE_SET_PITCH_Server, MSG_MOVE_SET_RUN_MODE_Server,
    MSG_MOVE_SET_WALK_MODE_Server, MSG_MOVE_START_BACKWARD_Server, MSG_MOVE_START_FORWARD_Server,
    MSG_MOVE_START_PITCH_DOWN_Server, MSG_MOVE_START_PITCH_UP_Server,
    MSG_MOVE_START_STRAFE_LEFT_Server, MSG_MOVE_START_STRAFE_RIGHT_Server,
    MSG_MOVE_START_SWIM_Server, MSG_MOVE_START_TURN_LEFT_Server, MSG_MOVE_START_TURN_RIGHT_Server,
    MSG_MOVE_STOP_PITCH_Server, MSG_MOVE_STOP_STRAFE_Server, MSG_MOVE_STOP_SWIM_Server,
    MSG_MOVE_STOP_Server, MSG_MOVE_STOP_TURN_Server,
};

use crate::protocol::geometry::position_from_vector;
use crate::world::Movement;

pub fn from_client(opcode: &ClientOpcodeMessage, guid: u64) -> Option<Movement> {
    let info = opcode.movement_info()?.clone();
    let position = position_from_vector(info.position, info.orientation);
    let packet = to_server_packet(opcode, Guid::new(guid), info)?;
    Some(Movement::new(guid, position, packet))
}

fn to_server_packet(
    opcode: &ClientOpcodeMessage,
    guid: Guid,
    info: wow_world_messages::vanilla::MovementInfo,
) -> Option<ServerOpcodeMessage> {
    Some(match opcode {
        ClientOpcodeMessage::MSG_MOVE_START_FORWARD(_) => {
            MSG_MOVE_START_FORWARD_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_START_BACKWARD(_) => {
            MSG_MOVE_START_BACKWARD_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_STOP(_) => MSG_MOVE_STOP_Server { guid, info }.into(),
        ClientOpcodeMessage::MSG_MOVE_START_STRAFE_LEFT(_) => {
            MSG_MOVE_START_STRAFE_LEFT_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_START_STRAFE_RIGHT(_) => {
            MSG_MOVE_START_STRAFE_RIGHT_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_STOP_STRAFE(_) => {
            MSG_MOVE_STOP_STRAFE_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_JUMP(_) => MSG_MOVE_JUMP_Server { guid, info }.into(),
        ClientOpcodeMessage::MSG_MOVE_START_TURN_LEFT(_) => {
            MSG_MOVE_START_TURN_LEFT_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_START_TURN_RIGHT(_) => {
            MSG_MOVE_START_TURN_RIGHT_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_STOP_TURN(_) => {
            MSG_MOVE_STOP_TURN_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_START_PITCH_UP(_) => {
            MSG_MOVE_START_PITCH_UP_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_START_PITCH_DOWN(_) => {
            MSG_MOVE_START_PITCH_DOWN_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_STOP_PITCH(_) => {
            MSG_MOVE_STOP_PITCH_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_SET_RUN_MODE(_) => {
            MSG_MOVE_SET_RUN_MODE_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_SET_WALK_MODE(_) => {
            MSG_MOVE_SET_WALK_MODE_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_FALL_LAND(_) => {
            MSG_MOVE_FALL_LAND_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_START_SWIM(_) => {
            MSG_MOVE_START_SWIM_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_STOP_SWIM(_) => {
            MSG_MOVE_STOP_SWIM_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_SET_FACING(_) => {
            MSG_MOVE_SET_FACING_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_SET_PITCH(_) => {
            MSG_MOVE_SET_PITCH_Server { guid, info }.into()
        }
        ClientOpcodeMessage::MSG_MOVE_HEARTBEAT(_) => {
            MSG_MOVE_HEARTBEAT_Server { guid, info }.into()
        }
        _ => return None,
    })
}
