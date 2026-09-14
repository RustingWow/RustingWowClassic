use crate::protocol::action::ClientAction;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    in_world: bool,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::QueryQuest { quest_id } => {
            let quest = lookup_quest(ctx.presence, quest_id).await?;
            ctx.connection.reply_quest(quest.as_ref()).await?;
        }
        ClientAction::QuestGiverStatus { guid } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_questgiver_status(ctx.presence, ctx.mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::QuestGiverStatus { .. } => {}
        ClientAction::QuestGiverHello { guid } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_questgiver_hello(ctx.presence, ctx.mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::QuestGiverHello { .. } => {}
        ClientAction::QuestGiverQuery { guid, quest_id } if in_world => {
            tracing::info!(guid, quest_id, "questgiver query");
            if let Some(character) = ctx.character.as_ref() {
                dispatch_quest_details(ctx.presence, ctx.mailbox, character.guid, guid, quest_id)
                    .await?;
            }
        }
        ClientAction::QuestGiverQuery { .. } => {}
        ClientAction::QuestGiverAccept { guid, quest_id } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_accept_quest(ctx.presence, ctx.mailbox, character.guid, guid, quest_id)
                    .await?;
            }
        }
        ClientAction::QuestGiverAccept { .. } => {}
        ClientAction::QuestGiverComplete { guid, quest_id } if in_world => {
            tracing::info!(guid, quest_id, "questgiver complete");
            if let Some(character) = ctx.character.as_ref() {
                dispatch_complete_quest(ctx.presence, ctx.mailbox, character.guid, guid, quest_id)
                    .await?;
            }
        }
        ClientAction::QuestGiverComplete { .. } => {}
        ClientAction::QuestGiverChooseReward {
            guid,
            quest_id,
            reward,
        } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_choose_quest_reward(
                    ctx.presence,
                    ctx.mailbox,
                    character.guid,
                    guid,
                    quest_id,
                    reward,
                )
                .await?;
            }
        }
        ClientAction::QuestGiverChooseReward { .. } => {}
        ClientAction::QuestLogRemove { slot } if in_world => {
            if let Some(character) = ctx.character.as_ref() {
                dispatch_abandon_quest(ctx.presence, ctx.mailbox, character.guid, slot).await?;
            }
        }
        ClientAction::QuestLogRemove { .. } => {}
        _ => unreachable!("quest::handle called with non-quest action"),
    }
    Ok(true)
}

async fn lookup_quest(
    presence: &Option<super::WorldPresence>,
    entry: u32,
) -> anyhow::Result<Option<crate::catalog::QuestRow>> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        super::MapBackend::Local(world) => Ok(crate::map_handle::MapHandle::quest(world, entry)),
        super::MapBackend::Remote(session) => session.quest(entry).await,
    }
}

async fn dispatch_questgiver_status(
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
            crate::map_handle::MapHandle::questgiver_status(world, mailbox, player, npc)
        }
        super::MapBackend::Remote(session) => session.questgiver_status(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_questgiver_hello(
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
            crate::map_handle::MapHandle::open_questgiver(world, mailbox, player, npc)
        }
        super::MapBackend::Remote(session) => session.open_questgiver(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_quest_details(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
    quest_id: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::query_quest_details(world, mailbox, player, npc, quest_id)
        }
        super::MapBackend::Remote(session) => {
            session.query_quest_details(player, npc, quest_id).await?
        }
    }
    Ok(())
}

async fn dispatch_accept_quest(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
    quest_id: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::accept_quest(world, mailbox, player, npc, quest_id)
        }
        super::MapBackend::Remote(session) => session.accept_quest(player, npc, quest_id).await?,
    }
    Ok(())
}

async fn dispatch_complete_quest(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
    quest_id: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::complete_quest(world, mailbox, player, npc, quest_id)
        }
        super::MapBackend::Remote(session) => session.complete_quest(player, npc, quest_id).await?,
    }
    Ok(())
}

async fn dispatch_choose_quest_reward(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    npc: u64,
    quest_id: u32,
    reward: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => crate::map_handle::MapHandle::choose_quest_reward(
            world, mailbox, player, npc, quest_id, reward,
        ),
        super::MapBackend::Remote(session) => {
            session
                .choose_quest_reward(player, npc, quest_id, reward)
                .await?
        }
    }
    Ok(())
}

async fn dispatch_abandon_quest(
    presence: &Option<super::WorldPresence>,
    mailbox: &crate::world::PlayerMailbox,
    player: u64,
    slot: u8,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        super::MapBackend::Local(world) => {
            crate::map_handle::MapHandle::abandon_quest(world, mailbox, player, slot)
        }
        super::MapBackend::Remote(session) => session.abandon_quest(player, slot).await?,
    }
    Ok(())
}
