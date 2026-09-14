use crate::protocol::action::ClientAction;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::Select { guid } => {
            ctx.gm.selection = if guid == 0 { None } else { Some(guid) };
        }
        _ => unreachable!("select::handle called with non-select action"),
    }
    Ok(true)
}
