use wow_shared::{CharacterMap, DbEnum, Position};

use super::{CommandCtx, CommandResult, Effect};

pub fn gps(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok(format!(
        "Map {} XYZ ({:.3}, {:.3}, {:.3}) O {:.3}",
        ctx.map_id, ctx.position.x, ctx.position.y, ctx.position.z, ctx.position.orientation
    ))
}

pub fn go_xyz(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(x) = args.first().and_then(|value| value.parse::<f32>().ok()) else {
        return CommandResult::err("Syntax: .go xyz $x $y [$z [$map [$o]]]");
    };
    let Some(y) = args.get(1).and_then(|value| value.parse::<f32>().ok()) else {
        return CommandResult::err("Syntax: .go xyz $x $y [$z [$map [$o]]]");
    };
    let z = args
        .get(2)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(ctx.position.z);
    let map_id = args
        .get(3)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(ctx.map_id);
    let orientation = args
        .get(4)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(ctx.position.orientation);
    if CharacterMap::from_protocol(map_id).is_none() {
        return CommandResult::err(format!("Unknown map {map_id}."));
    }
    CommandResult::ok(format!("Teleporting to {x:.1} {y:.1} {z:.1} map {map_id}.")).with(
        Effect::Teleport {
            map_id,
            position: Position {
                x,
                y,
                z,
                orientation,
            },
        },
    )
}

pub fn tele(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .tele $location");
    }
    let name = args.join(" ");
    let Some(location) = ctx.tele.get(&name) else {
        return CommandResult::err(format!("Unknown location '{name}'."));
    };
    CommandResult::ok(format!("Teleporting to {}.", location.name)).with(Effect::Teleport {
        map_id: location.map_id,
        position: location.position,
    })
}

pub fn tele_add(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(name) = args.first() else {
        return CommandResult::err("Syntax: .tele add $name");
    };
    CommandResult::ok(format!("Saving teleport '{name}'.")).with(Effect::TeleAdd {
        name: name.to_string(),
    })
}

pub fn tele_del(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(name) = args.first() else {
        return CommandResult::err("Syntax: .tele del $name");
    };
    CommandResult::ok(format!("Deleting teleport '{name}'.")).with(Effect::TeleDel {
        name: name.to_string(),
    })
}
pub fn appear(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(name) = args.first() else {
        return CommandResult::err("Syntax: .appear $player");
    };
    CommandResult::ok(format!("Appearing at {name}.")).with(Effect::Appear {
        name: name.to_string(),
    })
}

pub fn summon(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(name) = args.first() else {
        return CommandResult::err("Syntax: .summon $player");
    };
    CommandResult::ok(format!("Summoning {name}.")).with(Effect::Summon {
        name: name.to_string(),
    })
}

pub fn recall(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::ok("Returning to previous location.").with(Effect::Recall)
}
