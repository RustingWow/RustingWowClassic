use std::panic::{AssertUnwindSafe, catch_unwind};

use tokio::io::AsyncReadExt;
use wow_srp::vanilla_header::{CLIENT_HEADER_LENGTH, DecrypterHalf};
use wow_world_messages::vanilla::opcode_to_name;
use wow_world_messages::vanilla::opcodes::ClientOpcodeMessage;

/// Vanilla client opcode that `wow_world_messages` 0.3 panics on while inflating.
const CMSG_UPDATE_ACCOUNT_DATA: u32 = 0x020B;

pub enum Incoming {
    Message(ClientOpcodeMessage),
    Skipped {
        opcode: u32,
        name: Option<&'static str>,
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

    let name = opcode_to_name(header.opcode);
    if header.opcode == CMSG_UPDATE_ACCOUNT_DATA {
        return Ok(Incoming::Skipped {
            opcode: header.opcode,
            name,
        });
    }

    let mut raw = Vec::with_capacity(6 + body.len());
    raw.extend_from_slice(&header.size.to_be_bytes());
    raw.extend_from_slice(&header.opcode.to_le_bytes());
    raw.extend_from_slice(&body);

    match catch_unwind(AssertUnwindSafe(|| {
        ClientOpcodeMessage::read_unencrypted(&mut raw.as_slice())
    })) {
        Ok(Ok(message)) => Ok(Incoming::Message(message)),
        Ok(Err(_)) | Err(_) => Ok(Incoming::Skipped {
            opcode: header.opcode,
            name,
        }),
    }
}
