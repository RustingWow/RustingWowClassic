use super::{CommandCtx, CommandResult, Effect, parse_id};

pub fn modify_money(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(delta) = args.first().and_then(|value| value.parse::<i64>().ok()) else {
        return CommandResult::err("Syntax: .modify money $copper");
    };
    CommandResult::ok(format!("Money changed by {delta} copper.")).with(Effect::ModifyMoney(delta))
}

pub fn modify_hp(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(health) = args.first().and_then(|value| value.parse::<i32>().ok()) else {
        return CommandResult::err("Syntax: .modify hp $value");
    };
    let health = health.clamp(1, ctx.max_health);
    CommandResult::ok(format!("Health set to {health}.")).with(Effect::SetHp { health })
}

pub fn additem(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(entry) = args.first().and_then(|value| parse_id(value)) else {
        return CommandResult::err("Syntax: .additem $id [$count]");
    };
    let count = args
        .get(1)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);
    if let Some(catalog) = ctx.catalog
        && catalog.item(entry).is_none()
    {
        return CommandResult::err(format!("Unknown item {entry}."));
    }
    let name = ctx
        .catalog
        .and_then(|catalog| catalog.item(entry))
        .map(|item| item.name.as_str())
        .unwrap_or("item");
    CommandResult::ok(format!("Added {count}x {name} ({entry})."))
        .with(Effect::AddItem { entry, count })
}

pub fn revive(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("Revived.").with(Effect::Revive { guid: None })
}

pub fn die(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("Killed.").with(Effect::Die { guid: None })
}

pub fn pinfo(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let name = args.first().copied().unwrap_or(ctx.character_name);
    let online = ctx
        .online
        .iter()
        .find(|player| player.name.eq_ignore_ascii_case(name));
    let mut lines = vec![format!("Player: {name}")];
    if name.eq_ignore_ascii_case(ctx.character_name) {
        lines.push(format!(
            "Account: {} (id {}) gmlevel {}",
            ctx.account.username, ctx.account.id, ctx.account.gmlevel
        ));
        lines.push(format!("GUID: {}", ctx.character_guid));
        lines.push(format!(
            "HP {}/{} copper {}",
            ctx.health, ctx.max_health, ctx.copper
        ));
        lines.push(format!(
            "Map {} pos {:.1} {:.1} {:.1}",
            ctx.map_id, ctx.position.x, ctx.position.y, ctx.position.z
        ));
    } else if let Some(player) = online {
        lines.push(format!("Online on map {}", player.map_id));
        lines.push(format!("gmlevel {}", player.gmlevel));
    } else {
        lines.push("Player is not online.".to_string());
    }
    CommandResult::many(lines)
}

pub fn guid(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    match ctx.selection {
        Some(guid) => {
            if let Some(creature) = &ctx.selected_creature {
                CommandResult::ok(format!(
                    "Selected {} guid {} entry {}",
                    creature.name, creature.guid, creature.entry
                ))
            } else if let Some(name) = &ctx.selected_player_name {
                CommandResult::ok(format!("Selected {name} guid {guid}"))
            } else {
                CommandResult::ok(format!("Selected guid {guid}"))
            }
        }
        None => CommandResult::ok(format!(
            "No selection. Your guid is {}.",
            ctx.character_guid
        )),
    }
}

pub fn save(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("Player saved.").with(Effect::Save)
}

pub fn kick(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.first().is_some() {
        return CommandResult::err("Kicking other players is not implemented yet.");
    }
    CommandResult::ok("You have been kicked.").with(Effect::Kick)
}

pub fn announce(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .announce $message");
    }
    CommandResult::ok("Announcement sent.").with(Effect::Announce {
        text: args.join(" "),
        named: false,
        gm_only: false,
        notify: false,
    })
}

pub fn nameannounce(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .nameannounce $message");
    }
    CommandResult::ok("Announcement sent.").with(Effect::Announce {
        text: args.join(" "),
        named: true,
        gm_only: false,
        notify: false,
    })
}

pub fn notify(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .notify $message");
    }
    CommandResult::ok("Notification sent.").with(Effect::Announce {
        text: args.join(" "),
        named: false,
        gm_only: false,
        notify: true,
    })
}

pub fn gmannounce(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .gmannounce $message");
    }
    CommandResult::ok("GM announcement sent.").with(Effect::Announce {
        text: args.join(" "),
        named: false,
        gm_only: true,
        notify: false,
    })
}
