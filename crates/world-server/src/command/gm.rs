use super::{CommandCtx, CommandResult, Effect, Security};

pub fn gm_toggle(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if ctx.account.gmlevel < Security::Moderator as u8 {
        return CommandResult::err("You do not have access to this command.");
    }
    match args.first().copied() {
        Some("on") => gm_on(ctx, args),
        Some("off") => gm_off(ctx, args),
        None => {
            if ctx.gm_on {
                gm_off(ctx, args)
            } else {
                gm_on(ctx, args)
            }
        }
        Some(_) => CommandResult::err("Syntax: .gm [on/off]"),
    }
}

pub fn gm_on(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("GM mode is ON.").with(Effect::SetGmOn(true))
}

pub fn gm_off(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("GM mode is OFF.").with(Effect::SetGmOn(false))
}

pub fn gm_visible(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let visible = match args.first().copied() {
        Some("on") => true,
        Some("off") => false,
        None => !ctx.gm_visible,
        Some(_) => return CommandResult::err("Syntax: .gm visible [on/off]"),
    };
    let label = if visible { "ON" } else { "OFF" };
    CommandResult::ok(format!("GM visibility is {label}.")).with(Effect::SetGmVisible(visible))
}

pub fn gm_chat(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let on = match args.first().copied() {
        Some("on") => true,
        Some("off") => false,
        None => !ctx.gm_chat,
        Some(_) => return CommandResult::err("Syntax: .gm chat [on/off]"),
    };
    let label = if on { "ON" } else { "OFF" };
    CommandResult::ok(format!("GM chat badge is {label}.")).with(Effect::SetGmChat(on))
}

pub fn gm_ingame(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    let gms: Vec<_> = ctx
        .online
        .iter()
        .filter(|player| player.gmlevel > 0)
        .map(|player| format!("{} (gm{})", player.name, player.gmlevel))
        .collect();
    if gms.is_empty() {
        return CommandResult::ok("No GMs are currently in-game.");
    }
    CommandResult::ok(format!("GMs in-game: {}", gms.join(", ")))
}

pub fn gm_list(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("GM accounts:").with(Effect::GmList)
}
