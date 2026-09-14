use crate::protocol::action::ClientAction;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    in_world: bool,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::GossipHello { guid } if in_world => {
            tracing::info!(guid, "gossip hello");
            if let Some(character) = ctx.character.as_ref() {
                dispatch_gossip_hello(ctx.presence, ctx.mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::GossipHello { .. } => {}
        ClientAction::GossipSelect { guid, option } if in_world => {
            tracing::info!(guid, option, "gossip select");
            if let Some(character) = ctx.character.as_ref() {
                dispatch_gossip_select(ctx.presence, ctx.mailbox, character.guid, guid, option)
                    .await?;
            }
        }
        ClientAction::GossipSelect { .. } => {}
        ClientAction::QueryNpcText { text_id } => {
            let pages = npc_text_pages(ctx.presence, text_id).await?;
            ctx.connection.reply_npc_text(text_id, &pages).await?;
        }
        ClientAction::ListVendor { guid } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_list_vendor(ctx.presence, ctx.mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::ListVendor { .. } => {}
        ClientAction::BuyItem {
            vendor,
            item,
            amount,
        } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_buy_item(
                    ctx.presence,
                    ctx.mailbox,
                    character.guid,
                    vendor,
                    item,
                    amount,
                )
                .await?;
            }
        }
        ClientAction::BuyItem { .. } => {}
        _ => unreachable!("gossip::handle called with non-gossip action"),
    }
    Ok(true)
}

async fn npc_text_pages(
    presence: &Option<super::WorldPresence>,
    text_id: u32,
) -> anyhow::Result<[crate::catalog::NpcTextPage; 8]> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(std::array::from_fn(|_| {
            crate::catalog::NpcTextPage::default()
        }));
    };
    match backend {
        super::MapBackend::Local(world) => {
            Ok(crate::map_handle::MapHandle::npc_text_pages(world, text_id))
        }
        super::MapBackend::Remote(session) => session.npc_text_pages(text_id).await,
    }
}

async fn dispatch_gossip_hello(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::open_gossip(world, mailbox, player, npc)
        }
        super::MapBackend::Remote(session) => session.open_gossip(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_gossip_select(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
    option: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::select_gossip_option(world, mailbox, player, npc, option)
        }
        super::MapBackend::Remote(session) => {
            session.select_gossip_option(player, npc, option).await?
        }
    }
    Ok(())
}

async fn dispatch_list_vendor(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::list_vendor(world, mailbox, player, npc)
        }
        super::MapBackend::Remote(session) => session.list_vendor(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_buy_item(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
    item: u32,
    amount: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::buy_item(world, mailbox, player, npc, item, amount)
        }
        super::MapBackend::Remote(session) => session.buy_item(player, npc, item, amount).await?,
    }
    Ok(())
}
