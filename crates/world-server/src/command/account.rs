use super::{CommandCtx, CommandResult, Effect, Security};

pub fn account_show(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok(format!(
        "Account {} (id {}) gmlevel {}",
        ctx.account.username, ctx.account.id, ctx.account.gmlevel
    ))
}

pub fn account_set_gmlevel(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if ctx.account.gmlevel < Security::Admin as u8 {
        return CommandResult::err("You do not have access to this command.");
    }
    let Some(username) = args.first() else {
        return CommandResult::err("Syntax: .account set gmlevel $account $level");
    };
    let Some(level) = args.get(1).and_then(|value| value.parse::<u8>().ok()) else {
        return CommandResult::err("Syntax: .account set gmlevel $account $level");
    };
    if level > 3 {
        return CommandResult::err("GM level must be 0–3.");
    }
    CommandResult::ok(format!("Setting {username} gmlevel to {level}.")).with(Effect::SetGmLevel {
        username: username.to_string(),
        level,
    })
}
