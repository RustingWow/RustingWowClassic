use wow_shared::CreatureFaction;

use crate::catalog::CreatureTypeRow;

use super::{CommandCtx, CommandResult, CreatureInfo, Effect, parse_id};

pub fn npc_info(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    let Some(creature) = &ctx.selected_creature else {
        return CommandResult::err("You must select a creature.");
    };
    CommandResult::many(npc_info_lines(ctx, creature))
}

pub fn npc_add_temp(_ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let Some(entry) = args.first().and_then(|value| parse_id(value)) else {
        return CommandResult::err("Syntax: .npc add temp $entry");
    };
    CommandResult::ok(format!("Spawning temporary creature {entry}."))
        .with(Effect::SpawnNpc { entry })
}

pub fn npc_delete(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    let guid = args
        .first()
        .and_then(|value| value.parse::<u64>().ok())
        .or(ctx.selected_creature.as_ref().map(|creature| creature.guid));
    let Some(guid) = guid else {
        return CommandResult::err("Syntax: .npc delete [$guid] (or select a creature)");
    };
    CommandResult::ok(format!("Deleting creature {guid}.")).with(Effect::DeleteNpc { guid })
}

fn faction_label(id: i32) -> String {
    match CreatureFaction::from_protocol(id as u32) {
        Some(faction) => format!("{} ({id})", faction.as_str()),
        None => id.to_string(),
    }
}

fn npc_info_lines(ctx: &CommandCtx<'_>, creature: &CreatureInfo) -> Vec<String> {
    let mut lines = vec![
        format!(
            "{} entry {} guid {}",
            creature.name, creature.entry, creature.guid
        ),
        format!(
            "HP {}/{} level {}",
            creature.health, creature.max_health, creature.level
        ),
        format!(
            "XYZ ({:.3}, {:.3}, {:.3})",
            creature.position.x, creature.position.y, creature.position.z
        ),
        format!(
            "hostile {} wire faction {}",
            creature.hostile,
            faction_label(creature.faction)
        ),
    ];
    match ctx
        .catalog
        .and_then(|catalog| catalog.creature_type(creature.entry))
    {
        Some(kind) => lines.extend(template_lines(kind)),
        None => lines.push("No creature_types row in catalog.".into()),
    }
    lines
}

fn template_lines(kind: &CreatureTypeRow) -> Vec<String> {
    vec![
        format!(
            "template {} '{}' sub_name '{}'",
            kind.entry, kind.name, kind.sub_name
        ),
        format!("min_level {} max_level {}", kind.level, kind.max_level),
        format!("display_id {}", kind.display_id),
        format!("faction {}", faction_label(kind.faction)),
        format!(
            "family {} creature_type {}",
            kind.family, kind.creature_type
        ),
        format!(
            "npc_flags {} unit_flags {} civilian {}",
            kind.npc_flags, kind.unit_flags, kind.civilian
        ),
        format!(
            "health {} melee_damage {} loot_id {}",
            kind.health, kind.melee_damage, kind.loot_id
        ),
        format!(
            "skinning_loot_id {} pickpocket_loot_id {}",
            kind.skinning_loot_id, kind.pickpocket_loot_id
        ),
        format!(
            "gossip_menu_id {} vendor_template_id {}",
            kind.gossip_menu_id, kind.vendor_template_id
        ),
    ]
}
