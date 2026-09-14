use crate::protocol::action::ClientAction;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    in_world: bool,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::Attack { guid } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                tracing::info!(player = character.guid, target = guid, "CMSG_ATTACKSWING");
                dispatch_attack(ctx.presence, ctx.mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::Attack { .. } => {}
        ClientAction::StopAttack if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_stop_attack(ctx.presence, ctx.mailbox, character.guid).await?;
            }
        }
        ClientAction::StopAttack => {}
        ClientAction::ChangeStandState { state } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_stand(ctx.presence, ctx.mailbox, character.guid, state).await?;
            }
        }
        ClientAction::ChangeStandState { .. } => {}
        _ => unreachable!("combat::handle called with non-combat action"),
    }
    Ok(true)
}

async fn dispatch_attack(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    attacker: u64,
    target: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::start_attack(world, mailbox, attacker, target)
        }
        super::MapBackend::Remote(session) => session.start_attack(attacker, target).await?,
    }
    Ok(())
}

async fn dispatch_stop_attack(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    attacker: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::stop_attack(world, mailbox, attacker)
        }
        super::MapBackend::Remote(session) => session.stop_attack(attacker).await?,
    }
    Ok(())
}

async fn dispatch_stand(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    guid: u64,
    state: u8,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::change_stand_state(world, mailbox, guid, state)
        }
        super::MapBackend::Remote(session) => session.change_stand_state(guid, state).await?,
    }
    Ok(())
}
