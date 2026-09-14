use crate::command::is_command;
use crate::protocol::action::ClientAction;
use crate::world::ChatChannel;

use super::ActionContext;
use super::command;

pub(super) async fn handle(
    action: ClientAction,
    in_world: bool,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::Chat(pending) if in_world => {
            if is_command(&pending.text) {
                if let Some(character) = ctx.character.as_mut() {
                    let stay = command::execute_command(
                        pending.text,
                        ctx.connection,
                        ctx.account,
                        ctx.store,
                        character,
                        ctx.router,
                        ctx.mailbox,
                        ctx.presence,
                        ctx.services,
                        ctx.gm,
                    )
                    .await?;
                    return Ok(stay);
                }
            } else if let Some(character) = ctx.character.as_ref() {
                let mut chat = pending.bind(character.guid);
                chat.gm_tag = ctx.gm.chat;
                match &chat.channel {
                    ChatChannel::Whisper { to } => {
                        ctx.router.directory().whisper(
                            ctx.mailbox,
                            chat.speaker,
                            to.clone(),
                            chat.text,
                        );
                    }
                    _ => dispatch_speak(ctx.presence, ctx.mailbox, chat).await?,
                }
            }
        }
        ClientAction::Chat(_) => {}
        _ => unreachable!("chat::handle called with non-chat action"),
    }
    Ok(true)
}

async fn dispatch_speak(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    chat: crate::world::Chat,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::speak(world, mailbox, chat)
        }
        super::MapBackend::Remote(session) => session.speak(chat).await?,
    }
    Ok(())
}
