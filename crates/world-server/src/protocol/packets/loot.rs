use tokio::io::AsyncWriteExt;
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{SMSG_LOOT_RELEASE_RESPONSE, SMSG_LOOT_REMOVED, ServerMessage};

use crate::world::LootOffer;

const SMSG_LOOT_RESPONSE: u16 = 0x0160;
const LOOT_METHOD_ERROR: u8 = 0;
const LOOT_METHOD_CORPSE: u8 = 1;
const LOOT_SLOT_ALLOW: u8 = 0;
const MAX_LOOT_ITEMS: usize = 16;

pub async fn loot_opened<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    gold: u32,
    items: Vec<LootOffer>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let body = loot_success_body(guid, gold, &items);
    tracing::info!(
        guid,
        gold,
        items = items.len(),
        bytes = body.len(),
        "SMSG_LOOT_RESPONSE"
    );
    write_loot_response(stream, encrypter, &body).await
}

pub async fn loot_failed<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    error: u8,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    // CMaNGOS Player::SendLootError: guid + method 0 + error. No gold/count.
    let mut body = Vec::with_capacity(10);
    body.extend_from_slice(&guid.to_le_bytes());
    body.push(LOOT_METHOD_ERROR);
    body.push(error);
    tracing::info!(guid, error, "SMSG_LOOT_RESPONSE error");
    write_loot_response(stream, encrypter, &body).await
}

pub async fn loot_taken<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    index: u8,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_LOOT_REMOVED { slot: index }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn loot_closed<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_LOOT_RELEASE_RESPONSE {
        guid: Guid::new(guid),
        unknown1: 1,
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

async fn write_loot_response<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    body: &[u8],
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let mut header = Vec::with_capacity(4);
    encrypter.write_encrypted_server_header(
        &mut header,
        2 + body.len() as u16,
        SMSG_LOOT_RESPONSE,
    )?;
    stream.write_all(&header).await?;
    stream.write_all(body).await?;
    Ok(())
}

fn loot_success_body(guid: u64, gold: u32, items: &[LootOffer]) -> Vec<u8> {
    // wow_world_messages vanilla LootItem omits count/display/random; 1.12 needs CMaNGOS layout.
    let items = items.get(..items.len().min(MAX_LOOT_ITEMS)).unwrap_or(&[]);
    let mut body = Vec::with_capacity(14 + items.len() * 22);
    body.extend_from_slice(&guid.to_le_bytes());
    body.push(LOOT_METHOD_CORPSE);
    body.extend_from_slice(&gold.to_le_bytes());
    body.push(items.len() as u8);
    for offer in items {
        body.push(offer.index);
        body.extend_from_slice(&offer.item_id.to_le_bytes());
        body.extend_from_slice(&offer.count.to_le_bytes());
        body.extend_from_slice(&offer.display_id.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.push(LOOT_SLOT_ALLOW);
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(index: u8) -> LootOffer {
        LootOffer {
            index,
            item_id: 117,
            count: 2,
            display_id: 3486,
        }
    }

    #[test]
    fn empty_corpse_loot_is_fourteen_bytes() {
        assert_eq!(loot_success_body(1, 0, &[]).len(), 14);
    }

    #[test]
    fn one_item_uses_cmangos_slot_layout() {
        let body = loot_success_body(1, 0, &[offer(0)]);
        assert_eq!(body.len(), 36);
        assert_eq!(body[8], LOOT_METHOD_CORPSE);
        assert_eq!(body[13], 1);
        assert_eq!(body[14], 0);
        assert_eq!(&body[15..19], 117u32.to_le_bytes());
        assert_eq!(&body[19..23], 2u32.to_le_bytes());
        assert_eq!(&body[23..27], 3486u32.to_le_bytes());
        assert_eq!(body[35], LOOT_SLOT_ALLOW);
    }
}
