use super::{CommandCtx, CommandResult, Effect};

pub fn server_info(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok(format!(
        "WoWServer vanilla 1.12.1 — {} player(s) online.",
        ctx.player_count
    ))
}

pub fn server_motd(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok(format!("MOTD: {}", ctx.motd))
}

pub fn server_set_motd(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .server set motd $message");
    }
    let text = args.join(" ");
    CommandResult::ok("MOTD updated.").with(Effect::SetMotd(text))
}
