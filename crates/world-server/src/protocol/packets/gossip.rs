use tokio::io::AsyncWriteExt;
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    Gold, GossipItem, Language, ListInventoryItem, NpcTextUpdate, NpcTextUpdateEmote,
    SMSG_GOSSIP_COMPLETE, SMSG_GOSSIP_MESSAGE, SMSG_LIST_INVENTORY, SMSG_NPC_TEXT_UPDATE,
    ServerMessage,
};

use crate::catalog::NpcTextPage;
use crate::creature::GossipMenu;
use crate::quest::GossipQuestItem;
use crate::world::VendorOffer;

pub async fn gossip_opened<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    menu: GossipMenu,
    quests: Vec<GossipQuestItem>,
    pages: [NpcTextPage; 8],
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    npc_text_update(&mut *stream, encrypter, menu.text_id, &pages).await?;
    SMSG_GOSSIP_MESSAGE {
        guid: Guid::new(npc),
        title_text_id: menu.text_id,
        gossips: menu
            .options
            .into_iter()
            .map(|option| GossipItem {
                id: option.id,
                item_icon: option.icon.as_protocol(),
                coded: false,
                message: option.text,
            })
            .collect(),
        quests: quests.into_iter().map(super::quest::quest_item).collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn gossip_closed<W>(stream: &mut W, encrypter: &mut EncrypterHalf) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_GOSSIP_COMPLETE {}
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn vendor_list<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    items: Vec<VendorOffer>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_LIST_INVENTORY {
        vendor: Guid::new(npc),
        items: items
            .into_iter()
            .map(|offer| ListInventoryItem {
                item_stack_count: 1,
                item: offer.item_id,
                item_display_id: offer.display_id,
                max_items: offer.max_items,
                price: Gold::new(offer.price),
                max_durability: offer.max_durability,
                durability: offer.max_durability,
            })
            .collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn npc_text_update<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    text_id: u32,
    pages: &[NpcTextPage; 8],
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_NPC_TEXT_UPDATE {
        text_id,
        texts: npc_text_pages(pages),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

fn npc_text_pages(pages: &[NpcTextPage; 8]) -> [NpcTextUpdate; 8] {
    std::array::from_fn(|i| {
        let page = &pages[i];
        let male = page.text.clone();
        let female = if page.text_female.is_empty() {
            male.clone()
        } else {
            page.text_female.clone()
        };
        NpcTextUpdate {
            probability: page.probability,
            texts: [male, female],
            language: Language::Universal,
            emotes: [NpcTextUpdateEmote::default(); 3],
        }
    })
}
