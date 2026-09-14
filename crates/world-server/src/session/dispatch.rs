use crate::protocol::action::ClientAction;

use super::ActionContext;
use super::{character, chat, combat, gossip, loot, movement, query, quest, select};

pub(super) async fn handle_action(
    action: ClientAction,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    let in_world = ctx.presence.is_some();
    match action {
        ClientAction::Ping { .. }
        | ClientAction::ListCharacters
        | ClientAction::CreateCharacter { .. }
        | ClientAction::CreateCharacterRejected
        | ClientAction::DeleteCharacter { .. }
        | ClientAction::EnterWorld { .. }
        | ClientAction::LogoutRequest
        | ClientAction::PlayerLogout
        | ClientAction::LogoutCancel => character::handle(action, ctx).await,

        ClientAction::Attack { .. }
        | ClientAction::StopAttack
        | ClientAction::ChangeStandState { .. } => combat::handle(action, in_world, ctx).await,

        ClientAction::GossipHello { .. }
        | ClientAction::GossipSelect { .. }
        | ClientAction::QueryNpcText { .. }
        | ClientAction::ListVendor { .. }
        | ClientAction::BuyItem { .. } => gossip::handle(action, in_world, ctx).await,

        ClientAction::QueryQuest { .. }
        | ClientAction::QuestGiverStatus { .. }
        | ClientAction::QuestGiverHello { .. }
        | ClientAction::QuestGiverQuery { .. }
        | ClientAction::QuestGiverAccept { .. }
        | ClientAction::QuestGiverComplete { .. }
        | ClientAction::QuestGiverChooseReward { .. }
        | ClientAction::QuestLogRemove { .. } => quest::handle(action, in_world, ctx).await,

        ClientAction::Loot { .. } | ClientAction::LootItem { .. } | ClientAction::LootRelease => {
            loot::handle(action, in_world, ctx).await
        }

        ClientAction::Moved(_) => movement::handle(action, in_world, ctx).await,

        ClientAction::Chat(_) => chat::handle(action, in_world, ctx).await,

        ClientAction::QueryName { .. }
        | ClientAction::QueryCreature { .. }
        | ClientAction::QueryGameObject { .. }
        | ClientAction::QueryItem { .. } => query::handle(action, ctx).await,

        ClientAction::Select { .. } => select::handle(action, ctx).await,

        ClientAction::Noop => Ok(true),

        ClientAction::Ignored(ignored) => {
            if ctx.log_unhandled_packets {
                tracing::info!(
                    opcode = ignored.opcode,
                    name = ignored.name.as_deref(),
                    body_len = ignored.body_len,
                    in_world,
                    "unhandled packet"
                );
            }
            Ok(true)
        }
    }
}
