use crate::protocol::action::ClientAction;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    in_world: bool,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::Moved(pending) if in_world => {
            if let Some(character) = ctx.character.as_mut() {
                if let Some(movement) = pending.bind(character.guid) {
                    character.position = movement.position;
                    dispatch_move(ctx.presence, ctx.mailbox, movement).await?;
                }
            }
        }
        ClientAction::Moved(_) => {}
        _ => unreachable!("movement::handle called with non-movement action"),
    }
    Ok(true)
}

async fn dispatch_move(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    movement: crate::world::Movement,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::broadcast_move(world, mailbox, movement)
        }
        super::MapBackend::Remote(session) => {
            session
                .broadcast_move(movement.guid, movement.position)
                .await?;
        }
    }
    Ok(())
}
