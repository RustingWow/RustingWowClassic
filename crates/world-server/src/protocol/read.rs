use std::panic::{AssertUnwindSafe, catch_unwind};

use tokio::io::AsyncReadExt;
use wow_srp::vanilla_header::{CLIENT_HEADER_LENGTH, DecrypterHalf};
use wow_world_messages::Guid;
use wow_world_messages::vanilla::opcode_to_name;
use wow_world_messages::vanilla::opcodes::ClientOpcodeMessage;
use wow_world_messages::vanilla::{CMSG_GOSSIP_SELECT_OPTION, CMSG_QUESTGIVER_QUERY_QUEST};

/// Vanilla client opcode that `wow_world_messages` 0.3 panics on while inflating.
const CMSG_UPDATE_ACCOUNT_DATA: u32 = 0x020B;
const CMSG_GOSSIP_SELECT_OPTION: u32 = 0x017C;
const CMSG_QUESTGIVER_QUERY_QUEST: u32 = 0x0186;

pub enum Incoming {
    Message(ClientOpcodeMessage),
    Skipped {
        opcode: u32,
        name: Option<&'static str>,
        body_len: usize,
    },
}

pub async fn read_incoming<R>(
    stream: &mut R,
    decrypter: &mut DecrypterHalf,
) -> std::io::Result<Incoming>
where
    R: AsyncReadExt + Unpin,
{
    let mut header_bytes = [0_u8; CLIENT_HEADER_LENGTH as usize];
    stream.read_exact(&mut header_bytes).await?;
    let header = decrypter.decrypt_client_header(header_bytes);

    let body_len = usize::from(header.size.saturating_sub(4));
    let mut body = vec![0_u8; body_len];
    if body_len > 0 {
        stream.read_exact(&mut body).await?;
    }

    Ok(decode_incoming(header.opcode, &body))
}

pub(crate) fn decode_incoming(opcode: u32, body: &[u8]) -> Incoming {
    let name = opcode_to_name(opcode);
    if opcode == CMSG_UPDATE_ACCOUNT_DATA {
        return Incoming::Skipped {
            opcode,
            name,
            body_len: body.len(),
        };
    }
    if opcode == CMSG_GOSSIP_SELECT_OPTION {
        return match parse_gossip_select(body) {
            Some(message) => Incoming::Message(message),
            None => Incoming::Skipped {
                opcode,
                name,
                body_len: body.len(),
            },
        };
    }
    if opcode == CMSG_QUESTGIVER_QUERY_QUEST {
        return match parse_questgiver_query(body) {
            Some(message) => Incoming::Message(message),
            None => Incoming::Skipped {
                opcode,
                name,
                body_len: body.len(),
            },
        };
    }

    let mut raw = Vec::with_capacity(6 + body.len());
    raw.extend_from_slice(&(u16::try_from(body.len() + 4).unwrap_or(u16::MAX)).to_be_bytes());
    raw.extend_from_slice(&opcode.to_le_bytes());
    raw.extend_from_slice(body);

    match catch_unwind(AssertUnwindSafe(|| {
        ClientOpcodeMessage::read_unencrypted(&mut raw.as_slice())
    })) {
        Ok(Ok(message)) => Incoming::Message(message),
        Ok(Err(_)) | Err(_) => Incoming::Skipped {
            opcode,
            name,
            body_len: body.len(),
        },
    }
}

fn parse_gossip_select(body: &[u8]) -> Option<ClientOpcodeMessage> {
    if body.len() < 12 {
        return None;
    }
    let guid = Guid::new(u64::from_le_bytes(body[0..8].try_into().ok()?));
    let first = u32::from_le_bytes(body[8..12].try_into().ok()?);
    // 1.12 sends guid+option; some builds also send menu_id then option_id.
    let gossip_list_id = if body.len() == 16 {
        u32::from_le_bytes(body[12..16].try_into().ok()?)
    } else {
        first
    };
    Some(
        CMSG_GOSSIP_SELECT_OPTION {
            guid,
            gossip_list_id,
            unknown: None,
        }
        .into(),
    )
}

fn parse_questgiver_query(body: &[u8]) -> Option<ClientOpcodeMessage> {
    if body.len() < 12 {
        return None;
    }
    let guid = u64::from_le_bytes(body[0..8].try_into().ok()?);
    let quest_id = u32::from_le_bytes(body[8..12].try_into().ok()?);
    Some(
        CMSG_QUESTGIVER_QUERY_QUEST {
            guid: Guid::new(guid),
            quest_id,
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::action::ClientAction;

    #[test]
    fn query_quest_accepts_vanilla_12_byte_body() {
        let mut body = [0_u8; 12];
        body[0..8].copy_from_slice(&0xF130_0000_0000_0001_u64.to_le_bytes());
        body[8..12].copy_from_slice(&9_001_u32.to_le_bytes());

        match ClientAction::from(decode_incoming(CMSG_QUESTGIVER_QUERY_QUEST, &body)) {
            ClientAction::QuestGiverQuery { guid, quest_id } => {
                assert_eq!(guid, 0xF130_0000_0000_0001);
                assert_eq!(quest_id, 9_001);
            }
            _ => panic!("expected query"),
        }
    }

    #[test]
    fn query_quest_ignores_trailing_byte() {
        let mut body = [0_u8; 13];
        body[0..8].copy_from_slice(&42_u64.to_le_bytes());
        body[8..12].copy_from_slice(&83_u32.to_le_bytes());
        body[12] = 1;

        match ClientAction::from(decode_incoming(CMSG_QUESTGIVER_QUERY_QUEST, &body)) {
            ClientAction::QuestGiverQuery { guid, quest_id } => {
                assert_eq!(guid, 42);
                assert_eq!(quest_id, 83);
            }
            _ => panic!("expected query"),
        }
    }

    #[test]
    fn query_quest_shorter_than_12_is_skipped() {
        match decode_incoming(CMSG_QUESTGIVER_QUERY_QUEST, &[0; 8]) {
            Incoming::Skipped {
                opcode, body_len, ..
            } => {
                assert_eq!(opcode, CMSG_QUESTGIVER_QUERY_QUEST);
                assert_eq!(body_len, 8);
            }
            Incoming::Message(_) => panic!("expected skipped"),
        }
    }

    #[test]
    fn gossip_select_16_byte_body_uses_option_id() {
        let mut body = [0_u8; 16];
        body[0..8].copy_from_slice(&7_u64.to_le_bytes());
        body[8..12].copy_from_slice(&21_u32.to_le_bytes());
        body[12..16].copy_from_slice(&3_u32.to_le_bytes());

        match ClientAction::from(decode_incoming(CMSG_GOSSIP_SELECT_OPTION, &body)) {
            ClientAction::GossipSelect { guid, option } => {
                assert_eq!(guid, 7);
                assert_eq!(option, 3);
            }
            _ => panic!("expected gossip select"),
        }
    }
}
