use super::{CommandCtx, CommandResult};

pub fn lookup_tele(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .lookup tele $name");
    }
    let query = args.join(" ");
    let matches = ctx.tele.lookup(&query);
    if matches.is_empty() {
        return CommandResult::ok("No matching locations.");
    }
    CommandResult::many(
        matches
            .into_iter()
            .map(|loc| format!("{} (map {})", loc.name, loc.map_id))
            .collect(),
    )
}
pub fn lookup_item(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .lookup item $name");
    }
    let Some(catalog) = ctx.catalog else {
        return CommandResult::err("Item catalog is not loaded.");
    };
    let query = args.join(" ");
    let matches = catalog.lookup_items(&query);
    if matches.is_empty() {
        return CommandResult::ok("No matching items.");
    }
    CommandResult::many(
        matches
            .into_iter()
            .map(|item| format!("{} - {}", item.entry, item.name))
            .collect(),
    )
}

pub fn lookup_creature(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Syntax: .lookup creature $name");
    }
    let Some(catalog) = ctx.catalog else {
        return CommandResult::err("Creature catalog is not loaded.");
    };
    let query = args.join(" ");
    let matches = catalog.lookup_creatures(&query);
    if matches.is_empty() {
        return CommandResult::ok("No matching creatures.");
    }
    CommandResult::many(
        matches
            .into_iter()
            .map(|creature| format!("{} - {}", creature.entry, creature.name))
            .collect(),
    )
}
