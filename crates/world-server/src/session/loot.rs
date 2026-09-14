use crate::protocol::action::ClientAction;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    in_world: bool,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::Loot { guid } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                tracing::info!(player = character.guid, npc = guid, "CMSG_LOOT");
                dispatch_loot(ctx.presence, ctx.mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::Loot { .. } => {}
        ClientAction::LootItem { index } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_take_loot(ctx.presence, ctx.mailbox, character.guid, index).await?;
            }
        }
        ClientAction::LootItem { .. } => {}
        ClientAction::LootRelease if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_close_loot(ctx.presence, ctx.mailbox, character.guid).await?;
            }
        }
        ClientAction::LootRelease => {}
        _ => unreachable!("loot::handle called with non-loot action"),
    }
    Ok(true)
}

async fn dispatch_loot(
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
            crate::map_handle::MapHandle::open_loot(world, mailbox, player, npc)
        }
        super::MapBackend::Remote(session) => session.open_loot(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_take_loot(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    index: u8,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::take_loot(world, mailbox, player, 0, index)
        }
        super::MapBackend::Remote(session) => session.take_loot(player, index).await?,
    }
    Ok(())
}

async fn dispatch_close_loot(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::close_loot(world, mailbox, player)
        }
        super::MapBackend::Remote(session) => session.close_loot(player).await?,
    }
    Ok(())
}
