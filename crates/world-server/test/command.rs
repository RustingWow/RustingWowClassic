use super::*;
use wow_shared::{Account, Position};

fn account(gmlevel: u8) -> Account {
    Account {
        id: 1,
        username: "GMTEST".into(),
        gmlevel,
    }
}

fn ctx<'a>(account: &'a Account, tele: &'a TeleStore, online: &'a [OnlineInfo]) -> CommandCtx<'a> {
    CommandCtx {
        account,
        character_name: "Hero",
        character_guid: 1,
        map_id: 0,
        position: Position::NORTHSHIRE,
        copper: 10_000,
        health: 100,
        max_health: 100,
        selection: None,
        selected_creature: None,
        selected_player_name: None,
        gm_on: false,
        gm_visible: true,
        gm_chat: false,
        catalog: None,
        tele,
        online,
        motd: "Welcome.",
        player_count: online.len(),
    }
}

#[test]
fn player_cannot_use_gps() {
    let tele = TeleStore::memory();
    let account = account(0);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".gps", &ctx);
    assert_eq!(
        result.messages[0],
        "You do not have access to this command."
    );
    assert!(result.effects.is_empty());
}

#[test]
fn moderator_gps_reports_coordinates() {
    let tele = TeleStore::memory();
    let account = account(1);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".gps", &ctx);
    assert!(result.messages[0].contains("Map 0"));
    assert!(result.messages[0].contains("-8949.950"));
}

#[test]
fn stub_command_is_registered() {
    let tele = TeleStore::memory();
    let account = account(3);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".learn 133", &ctx);
    assert_eq!(result.messages[0], "Command not implemented yet.");
}

#[test]
fn unknown_command() {
    let tele = TeleStore::memory();
    let account = account(3);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".notacommand", &ctx);
    assert_eq!(result.messages[0], "There is no such command.");
}

#[test]
fn go_xyz_emits_teleport() {
    let tele = TeleStore::memory();
    let account = account(1);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".go 1 2 3", &ctx);
    assert_eq!(
        result.effects,
        vec![Effect::Teleport {
            map_id: 0,
            position: Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: 0.0,
            },
        }]
    );
}

#[test]
fn tele_stormwind() {
    let tele = TeleStore::memory();
    let account = account(1);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".tele Stormwind", &ctx);
    assert_eq!(result.effects.len(), 1);
    match &result.effects[0] {
        Effect::Teleport { map_id, .. } => assert_eq!(*map_id, 0),
        other => panic!("expected teleport, got {other:?}"),
    }
}

#[test]
fn modify_money_and_additem() {
    let tele = TeleStore::memory();
    let account = account(3);
    let ctx = ctx(&account, &tele, &[]);
    let money = dispatch(".modify money 500", &ctx);
    assert_eq!(money.effects, vec![Effect::ModifyMoney(500)]);
    let item = dispatch(".additem 25 2", &ctx);
    assert_eq!(
        item.effects,
        vec![Effect::AddItem {
            entry: 25,
            count: 2
        }]
    );
}

#[test]
fn chat_starting_with_dot_is_a_command() {
    assert!(is_command(".gm on"));
    assert!(is_command("!gps"));
    assert!(!is_command("hello .gm"));
}

#[test]
fn help_lists_player_commands() {
    let tele = TeleStore::memory();
    let account = account(0);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".help", &ctx);
    assert!(result.messages.iter().any(|line| line.contains(".help")));
    assert!(result.messages.iter().any(|line| line.contains(".account")));
    assert!(!result.messages.iter().any(|line| line.contains(".gps")));
}

fn creature_info() -> CreatureInfo {
    CreatureInfo {
        guid: 42,
        entry: 299,
        name: "Young Wolf".into(),
        position: Position::NORTHSHIRE,
        health: 45,
        max_health: 45,
        level: 1,
        faction: 38,
        hostile: true,
    }
}

fn wolf_template() -> crate::catalog::CreatureTypeRow {
    crate::catalog::CreatureTypeRow {
        entry: 299,
        name: "Young Wolf".into(),
        sub_name: String::new(),
        level: 1,
        max_level: 2,
        display_id: 447,
        faction: 32,
        family: 1,
        creature_type: 1,
        npc_flags: 0,
        unit_flags: 0x8,
        civilian: false,
        health: 45,
        melee_damage: 4,
        loot_id: 117,
        gossip_menu_id: 900001,
        vendor_template_id: 0,
        skinning_loot_id: 0,
        pickpocket_loot_id: 0,
    }
}

#[test]
fn npc_info_requires_a_creature_selection() {
    let tele = TeleStore::memory();
    let account = account(2);
    let ctx = ctx(&account, &tele, &[]);
    let result = dispatch(".npc info", &ctx);
    assert_eq!(result.messages[0], "You must select a creature.");
}

#[test]
fn npc_info_dumps_live_state_and_template() {
    let tele = TeleStore::memory();
    let account = account(2);
    let mut catalog = crate::catalog::Catalog::empty();
    catalog.types.insert(299, wolf_template());
    let mut ctx = ctx(&account, &tele, &[]);
    ctx.selected_creature = Some(creature_info());
    ctx.catalog = Some(&catalog);
    let result = dispatch(".npc info", &ctx);
    let text = result.messages.join("\n");
    assert!(text.contains("Young Wolf entry 299 guid 42"), "{text}");
    assert!(text.contains("hostile true"), "{text}");
    assert!(
        text.contains("BEAST_WOLF_38 (38)") || text.contains("wire faction"),
        "{text}"
    );
    assert!(
        text.contains("faction BEAST_WOLF (32)") || text.contains("faction 32"),
        "{text}"
    );
    assert!(text.contains("npc_flags 0"), "{text}");
    assert!(
        text.contains("unit_flags 8") || text.contains("unit_flags 0x8"),
        "{text}"
    );
    assert!(text.contains("civilian false"), "{text}");
    assert!(text.contains("loot_id 117"), "{text}");
    assert!(text.contains("gossip_menu_id 900001"), "{text}");
}
