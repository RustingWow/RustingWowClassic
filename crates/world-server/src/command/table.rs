use super::{Command, Security, handlers, live, nested, stub};

pub const ROOT: &[Command] = &[
    live(
        "help",
        &[],
        Security::Player,
        "Show command help",
        handlers::help,
    ),
    live(
        "commands",
        &[],
        Security::Player,
        "List commands for your level",
        handlers::commands,
    ),
    nested(
        "account",
        &[],
        Security::Player,
        "Account commands",
        ACCOUNT,
        Some(handlers::account_show),
    ),
    nested(
        "gm",
        &[],
        Security::Player,
        "GM mode commands",
        GM,
        Some(handlers::gm_toggle),
    ),
    stub("whispers", &[], Security::Moderator, "Toggle GM whispers"),
    live(
        "gps",
        &[],
        Security::Moderator,
        "Print current coordinates",
        handlers::gps,
    ),
    nested(
        "go",
        &[],
        Security::Moderator,
        "Teleport to coordinates or objects",
        GO,
        Some(handlers::go_xyz),
    ),
    nested(
        "tele",
        &[],
        Security::Moderator,
        "Teleport to a named location",
        TELE,
        Some(handlers::tele),
    ),
    live(
        "appear",
        &["goname"],
        Security::Moderator,
        "Teleport to a player",
        handlers::appear,
    ),
    live(
        "summon",
        &["namego"],
        Security::Moderator,
        "Summon a player",
        handlers::summon,
    ),
    stub(
        "groupsummon",
        &[],
        Security::Moderator,
        "Summon a player's group",
    ),
    live(
        "recall",
        &[],
        Security::Moderator,
        "Undo last GM teleport",
        handlers::recall,
    ),
    stub("unstuck", &["start"], Security::Player, "Unstuck a player"),
    nested(
        "modify",
        &[],
        Security::Moderator,
        "Modify player values",
        MODIFY,
        None,
    ),
    live(
        "money",
        &[],
        Security::Moderator,
        "Add or remove copper",
        handlers::modify_money,
    ),
    stub(
        "speed",
        &["aspeed"],
        Security::Moderator,
        "Modify movement speed",
    ),
    stub("waterwalk", &[], Security::GameMaster, "Walk on water"),
    stub(
        "taxicheat",
        &[],
        Security::GameMaster,
        "Unlock all flight paths",
    ),
    live(
        "additem",
        &[],
        Security::Admin,
        "Add an item to your bags",
        handlers::additem,
    ),
    stub("additemset", &[], Security::Admin, "Add an item set"),
    live(
        "revive",
        &[],
        Security::GameMaster,
        "Revive target or self",
        handlers::revive,
    ),
    live(
        "die",
        &[],
        Security::GameMaster,
        "Kill target or self",
        handlers::die,
    ),
    live(
        "pinfo",
        &[],
        Security::GameMaster,
        "Show player info",
        handlers::pinfo,
    ),
    live(
        "guid",
        &[],
        Security::GameMaster,
        "Show selected unit GUID",
        handlers::guid,
    ),
    live(
        "save",
        &[],
        Security::Player,
        "Save the current character",
        handlers::save,
    ),
    stub("saveall", &[], Security::Moderator, "Save all characters"),
    live(
        "kick",
        &[],
        Security::GameMaster,
        "Kick a player",
        handlers::kick,
    ),
    stub("mute", &[], Security::Moderator, "Mute an account"),
    stub("unmute", &[], Security::Moderator, "Unmute an account"),
    stub("freeze", &[], Security::Moderator, "Freeze a player"),
    stub("unfreeze", &[], Security::Moderator, "Unfreeze a player"),
    stub(
        "listfreeze",
        &[],
        Security::Moderator,
        "List frozen players",
    ),
    live(
        "announce",
        &[],
        Security::Moderator,
        "Broadcast a server message",
        handlers::announce,
    ),
    live(
        "nameannounce",
        &[],
        Security::Moderator,
        "Broadcast with your name",
        handlers::nameannounce,
    ),
    live(
        "notify",
        &[],
        Security::Moderator,
        "On-screen broadcast",
        handlers::notify,
    ),
    live(
        "gmannounce",
        &[],
        Security::Moderator,
        "Broadcast to GMs",
        handlers::gmannounce,
    ),
    stub(
        "gmnameannounce",
        &[],
        Security::Moderator,
        "Named GM broadcast",
    ),
    stub(
        "gmnotify",
        &[],
        Security::Moderator,
        "On-screen GM broadcast",
    ),
    nested(
        "npc",
        &[],
        Security::Moderator,
        "Creature commands",
        NPC,
        None,
    ),
    nested(
        "gobject",
        &["gameobject"],
        Security::GameMaster,
        "GameObject commands",
        GOBJECT,
        None,
    ),
    nested("quest", &[], Security::Admin, "Quest commands", QUEST, None),
    nested(
        "lookup",
        &[],
        Security::Moderator,
        "Search catalogs",
        LOOKUP,
        None,
    ),
    nested(
        "list",
        &[],
        Security::Admin,
        "List world objects",
        LIST,
        None,
    ),
    nested(
        "learn",
        &[],
        Security::Moderator,
        "Teach spells",
        LEARN,
        Some(handlers::unimplemented),
    ),
    stub("unlearn", &[], Security::Admin, "Unlearn a spell"),
    stub("aura", &[], Security::Admin, "Apply an aura"),
    stub("unaura", &[], Security::Admin, "Remove an aura"),
    nested("cast", &[], Security::Admin, "Cast a spell", CAST, None),
    stub("cooldown", &[], Security::Admin, "Clear cooldowns"),
    stub("levelup", &[], Security::Admin, "Add character levels"),
    nested(
        "character",
        &[],
        Security::GameMaster,
        "Character admin",
        CHARACTER,
        None,
    ),
    nested(
        "reset",
        &[],
        Security::Admin,
        "Reset character data",
        RESET,
        None,
    ),
    nested(
        "cheat",
        &[],
        Security::GameMaster,
        "Cheat toggles",
        CHEAT,
        None,
    ),
    stub("maxskill", &[], Security::Admin, "Max skills for level"),
    stub("setskill", &[], Security::Admin, "Set a skill"),
    stub("repairitems", &[], Security::GameMaster, "Repair all gear"),
    stub("bank", &[], Security::Admin, "Open the bank"),
    stub("dismount", &[], Security::Player, "Dismount"),
    stub("combatstop", &[], Security::GameMaster, "Leave combat"),
    stub("morph", &[], Security::GameMaster, "Change display id"),
    stub("demorph", &[], Security::GameMaster, "Restore display id"),
    nested(
        "server",
        &[],
        Security::Player,
        "Server commands",
        SERVER,
        None,
    ),
    nested(
        "send",
        &[],
        Security::Moderator,
        "Send mail or messages",
        SEND,
        None,
    ),
    nested(
        "reload",
        &[],
        Security::Admin,
        "Reload DB tables",
        RELOAD,
        None,
    ),
    nested("ban", &[], Security::Admin, "Ban commands", BAN, None),
    nested("unban", &[], Security::Admin, "Unban commands", UNBAN, None),
    nested(
        "baninfo",
        &[],
        Security::Admin,
        "Ban information",
        BANINFO,
        None,
    ),
    nested("banlist", &[], Security::Admin, "List bans", BANLIST, None),
    nested(
        "ticket",
        &[],
        Security::Moderator,
        "GM tickets",
        TICKET,
        None,
    ),
    nested(
        "debug",
        &[],
        Security::Moderator,
        "Debug commands",
        DEBUG,
        None,
    ),
    nested(
        "instance",
        &[],
        Security::Admin,
        "Instance commands",
        INSTANCE,
        None,
    ),
    nested(
        "honor",
        &[],
        Security::GameMaster,
        "Honor commands",
        HONOR,
        None,
    ),
    nested(
        "guild",
        &[],
        Security::GameMaster,
        "Guild admin",
        GUILD,
        None,
    ),
    nested("group", &[], Security::Admin, "Group admin", GROUP, None),
    nested("pet", &[], Security::GameMaster, "Pet commands", PET, None),
    nested("wp", &[], Security::GameMaster, "Waypoint editor", WP, None),
    stub("wpgps", &[], Security::Admin, "Dump GPS as waypoint SQL"),
    nested(
        "event",
        &[],
        Security::GameMaster,
        "World events",
        EVENT,
        None,
    ),
    nested(
        "disable",
        &[],
        Security::Admin,
        "Runtime disables",
        DISABLE,
        None,
    ),
    nested(
        "pdump",
        &[],
        Security::Admin,
        "Character dump files",
        PDUMP,
        None,
    ),
    stub(
        "cometome",
        &[],
        Security::Admin,
        "Move selected creature here",
    ),
    stub("respawn", &[], Security::Admin, "Respawn nearby creatures"),
    stub("damage", &[], Security::Admin, "Deal damage"),
    stub("distance", &[], Security::Admin, "Distance to target"),
    stub("movegens", &[], Security::Admin, "Movement generators"),
    stub("possess", &[], Security::Admin, "Possess a unit"),
    stub("unpossess", &[], Security::Admin, "End possess"),
    stub("bindsight", &[], Security::Admin, "Bind camera to a unit"),
    stub("unbindsight", &[], Security::Admin, "Unbind camera"),
    stub("hidearea", &[], Security::Admin, "Hide an explored area"),
    stub("showarea", &[], Security::Admin, "Show an explored area"),
    stub("linkgrave", &[], Security::Admin, "Link a graveyard"),
    stub("neargrave", &[], Security::Admin, "Nearest graveyard"),
    stub("wchange", &[], Security::Admin, "Change weather"),
    stub(
        "playall",
        &[],
        Security::GameMaster,
        "Play a sound for everyone",
    ),
    stub("channel", &[], Security::Admin, "Chat channel ownership"),
];

const ACCOUNT: &[Command] = &[
    stub("lock", &[], Security::Player, "Lock the account to this IP"),
    stub("password", &[], Security::Player, "Change your password"),
    nested(
        "set",
        &[],
        Security::Admin,
        "Set account fields",
        ACCOUNT_SET,
        None,
    ),
    stub("create", &[], Security::Admin, "Create an account"),
    stub("delete", &[], Security::Admin, "Delete an account"),
    stub("onlinelist", &[], Security::Admin, "List online accounts"),
];

const ACCOUNT_SET: &[Command] = &[
    live(
        "gmlevel",
        &[],
        Security::Admin,
        "Set another account's GM level",
        handlers::account_set_gmlevel,
    ),
    stub(
        "password",
        &[],
        Security::Admin,
        "Set another account's password",
    ),
    stub("addon", &[], Security::Admin, "Set expansion"),
];

const GM: &[Command] = &[
    live(
        "on",
        &[],
        Security::Moderator,
        "Enable GM mode",
        handlers::gm_on,
    ),
    live(
        "off",
        &[],
        Security::Moderator,
        "Disable GM mode",
        handlers::gm_off,
    ),
    live(
        "visible",
        &[],
        Security::Moderator,
        "Toggle GM visibility",
        handlers::gm_visible,
    ),
    live(
        "chat",
        &[],
        Security::Moderator,
        "Toggle GM chat badge",
        handlers::gm_chat,
    ),
    stub(
        "fly",
        &[],
        Security::GameMaster,
        "Toggle GM fly (not in 1.12)",
    ),
    live(
        "ingame",
        &[],
        Security::Player,
        "List GMs in-game",
        handlers::gm_ingame,
    ),
    live(
        "list",
        &[],
        Security::Admin,
        "List GM accounts",
        handlers::gm_list,
    ),
];

const GO: &[Command] = &[
    live(
        "xyz",
        &[],
        Security::Moderator,
        "Teleport to coordinates",
        handlers::go_xyz,
    ),
    stub(
        "creature",
        &[],
        Security::Moderator,
        "Teleport to a creature",
    ),
    stub(
        "object",
        &["gameobject"],
        Security::Moderator,
        "Teleport to a gameobject",
    ),
    stub(
        "graveyard",
        &[],
        Security::Moderator,
        "Teleport to a graveyard",
    ),
    stub(
        "taxinode",
        &[],
        Security::Moderator,
        "Teleport to a flight node",
    ),
    stub(
        "trigger",
        &["areatrigger"],
        Security::Moderator,
        "Teleport to an areatrigger",
    ),
    stub("grid", &[], Security::Moderator, "Teleport to a map grid"),
    stub(
        "zonexy",
        &[],
        Security::Moderator,
        "Teleport using zone coordinates",
    ),
    stub("ticket", &[], Security::Moderator, "Teleport to a ticket"),
];

const TELE: &[Command] = &[
    live(
        "add",
        &[],
        Security::Admin,
        "Save a named teleport",
        handlers::tele_add,
    ),
    live(
        "del",
        &[],
        Security::Admin,
        "Delete a named teleport",
        handlers::tele_del,
    ),
    stub(
        "name",
        &[],
        Security::Moderator,
        "Teleport a player to a location",
    ),
    stub(
        "group",
        &[],
        Security::Moderator,
        "Teleport a group to a location",
    ),
];

const MODIFY: &[Command] = &[
    live(
        "money",
        &[],
        Security::Moderator,
        "Add or remove copper",
        handlers::modify_money,
    ),
    live(
        "hp",
        &[],
        Security::Moderator,
        "Set health",
        handlers::modify_hp,
    ),
    stub("mana", &[], Security::Moderator, "Set mana"),
    stub("rage", &[], Security::Moderator, "Set rage"),
    stub("energy", &[], Security::Moderator, "Set energy"),
    nested(
        "speed",
        &[],
        Security::Moderator,
        "Modify speed",
        MODIFY_SPEED,
        Some(handlers::unimplemented),
    ),
    stub("scale", &[], Security::Moderator, "Change size"),
    stub("honor", &[], Security::Moderator, "Add honor"),
    stub("drunk", &[], Security::Moderator, "Set drunk state"),
    stub("gender", &[], Security::GameMaster, "Change gender"),
    stub(
        "reputation",
        &["rep"],
        Security::GameMaster,
        "Set reputation",
    ),
    stub("faction", &[], Security::GameMaster, "Set faction"),
    stub("mount", &[], Security::Moderator, "Visual mount"),
    stub("standstate", &[], Security::GameMaster, "Standing emote"),
    stub(
        "bit",
        &[],
        Security::Moderator,
        "Toggle an update-field bit",
    ),
    stub("morph", &[], Security::GameMaster, "Change display id"),
];

const MODIFY_SPEED: &[Command] = &[
    stub("walk", &[], Security::Moderator, "Walk speed"),
    stub("run", &[], Security::Moderator, "Run speed"),
    stub("swim", &[], Security::Moderator, "Swim speed"),
    stub(
        "backwalk",
        &["bwalk"],
        Security::Moderator,
        "Backwards walk speed",
    ),
    stub("fly", &[], Security::Moderator, "Fly speed"),
    stub(
        "all",
        &["aspeed"],
        Security::Moderator,
        "All movement speeds",
    ),
];

const NPC: &[Command] = &[
    nested(
        "add",
        &[],
        Security::GameMaster,
        "Spawn a creature",
        NPC_ADD,
        Some(handlers::unimplemented),
    ),
    live(
        "delete",
        &[],
        Security::GameMaster,
        "Delete a creature spawn",
        handlers::npc_delete,
    ),
    live(
        "info",
        &[],
        Security::GameMaster,
        "Show selected creature info",
        handlers::npc_info,
    ),
    stub("move", &[], Security::GameMaster, "Move spawn to you"),
    stub("follow", &[], Security::GameMaster, "Creature follows you"),
    stub("say", &[], Security::Moderator, "Make the NPC say"),
    stub("yell", &[], Security::Moderator, "Make the NPC yell"),
    stub("whisper", &[], Security::Moderator, "Make the NPC whisper"),
    stub("textemote", &[], Security::Moderator, "NPC text emote"),
    stub("playemote", &[], Security::Moderator, "NPC play emote"),
    nested(
        "set",
        &[],
        Security::GameMaster,
        "Set creature fields",
        NPC_SET,
        None,
    ),
    stub("additem", &[], Security::GameMaster, "Add a vendor item"),
    stub(
        "delitem",
        &["deleteitem"],
        Security::GameMaster,
        "Remove a vendor item",
    ),
];

const NPC_ADD: &[Command] = &[
    live(
        "temp",
        &[],
        Security::GameMaster,
        "Temporary creature spawn",
        handlers::npc_add_temp,
    ),
    stub("item", &[], Security::GameMaster, "Add vendor item"),
    stub("move", &[], Security::GameMaster, "Add a waypoint"),
    stub("formation", &[], Security::GameMaster, "Add formation"),
];

const NPC_SET: &[Command] = &[
    stub("level", &[], Security::GameMaster, "Set creature level"),
    stub("model", &[], Security::GameMaster, "Set display id"),
    stub("factionid", &[], Security::GameMaster, "Set faction"),
    stub("flag", &[], Security::GameMaster, "Set npc flags"),
    stub("movetype", &[], Security::GameMaster, "Set movement type"),
    stub(
        "spawndist",
        &[],
        Security::GameMaster,
        "Set wander distance",
    ),
    stub("spawntime", &[], Security::GameMaster, "Set respawn time"),
    stub("link", &[], Security::GameMaster, "Link respawn"),
    stub("entry", &[], Security::Admin, "Swap creature template"),
    stub("allowmove", &[], Security::Admin, "Allow creature movement"),
    stub("deathstate", &[], Security::GameMaster, "Set death state"),
];

const GOBJECT: &[Command] = &[
    nested(
        "add",
        &[],
        Security::GameMaster,
        "Spawn a gameobject",
        GOBJECT_ADD,
        Some(handlers::unimplemented),
    ),
    stub("delete", &[], Security::GameMaster, "Delete a gameobject"),
    stub("info", &[], Security::GameMaster, "Gameobject info"),
    stub("near", &[], Security::GameMaster, "List nearby gameobjects"),
    stub(
        "target",
        &[],
        Security::GameMaster,
        "Nearest matching gameobject",
    ),
    stub("move", &[], Security::GameMaster, "Move a gameobject"),
    stub("turn", &[], Security::GameMaster, "Rotate a gameobject"),
    stub(
        "activate",
        &[],
        Security::GameMaster,
        "Activate a gameobject",
    ),
];

const GOBJECT_ADD: &[Command] = &[stub(
    "temp",
    &[],
    Security::GameMaster,
    "Temporary gameobject spawn",
)];

const QUEST: &[Command] = &[
    stub("add", &[], Security::Admin, "Add a quest to the log"),
    stub(
        "complete",
        &[],
        Security::Admin,
        "Complete quest objectives",
    ),
    stub("remove", &[], Security::Admin, "Remove a quest"),
];

const LOOKUP: &[Command] = &[
    live(
        "item",
        &[],
        Security::Admin,
        "Search items",
        handlers::lookup_item,
    ),
    live(
        "creature",
        &[],
        Security::Admin,
        "Search creatures",
        handlers::lookup_creature,
    ),
    live(
        "tele",
        &[],
        Security::Moderator,
        "Search teleports",
        handlers::lookup_tele,
    ),
    stub("object", &[], Security::Admin, "Search gameobjects"),
    stub("quest", &[], Security::Admin, "Search quests"),
    stub("spell", &[], Security::Admin, "Search spells"),
    stub("skill", &[], Security::Admin, "Search skills"),
    stub("area", &[], Security::Moderator, "Search areas"),
    stub("faction", &[], Security::Admin, "Search factions"),
    stub("taxinode", &[], Security::Admin, "Search flight nodes"),
    stub(
        "itemset",
        &["item set"],
        Security::Admin,
        "Search item sets",
    ),
    nested(
        "player",
        &[],
        Security::GameMaster,
        "Find players",
        LOOKUP_PLAYER,
        None,
    ),
];

const LOOKUP_PLAYER: &[Command] = &[
    stub("account", &[], Security::GameMaster, "Find by account"),
    stub("ip", &[], Security::GameMaster, "Find by IP"),
    stub("email", &[], Security::GameMaster, "Find by email"),
];

const LIST: &[Command] = &[
    stub("item", &[], Security::Admin, "Who owns this item"),
    stub("creature", &[], Security::Admin, "List creature spawns"),
    stub("object", &[], Security::Admin, "List gameobject spawns"),
    stub("auras", &[], Security::Admin, "List auras on target"),
];

const LEARN: &[Command] = &[nested(
    "all",
    &[],
    Security::Moderator,
    "Learn groups of spells",
    LEARN_ALL,
    None,
)];

const LEARN_ALL: &[Command] = &[
    stub("my", &[], Security::Admin, "Learn your class kit"),
    stub("class", &[], Security::Admin, "Learn class spells"),
    stub("spells", &[], Security::Admin, "Learn all spells"),
    stub("talents", &[], Security::Admin, "Learn all talents"),
    stub(
        "default",
        &[],
        Security::Moderator,
        "Learn racial/class defaults",
    ),
    stub("lang", &[], Security::Moderator, "Learn all languages"),
    stub("crafts", &[], Security::GameMaster, "Learn professions"),
    stub("recipes", &[], Security::GameMaster, "Learn recipes"),
    stub(
        "blizzard",
        &["gm"],
        Security::GameMaster,
        "Learn GM utility spells",
    ),
];

const CAST: &[Command] = &[
    stub("self", &[], Security::Admin, "Cast on self"),
    stub("back", &[], Security::Admin, "Cast back"),
    stub("target", &[], Security::Admin, "Cast on target"),
    stub("dest", &[], Security::Admin, "Cast at destination"),
    stub("dist", &[], Security::Admin, "Cast at distance"),
];

const CHARACTER: &[Command] = &[
    stub("level", &[], Security::Admin, "Set character level"),
    stub(
        "rename",
        &[],
        Security::GameMaster,
        "Flag character for rename",
    ),
    stub(
        "erase",
        &[],
        Security::Admin,
        "Permanently delete a character",
    ),
    nested(
        "deleted",
        &[],
        Security::Admin,
        "Deleted character recycle bin",
        CHARACTER_DELETED,
        None,
    ),
    stub(
        "reputation",
        &[],
        Security::GameMaster,
        "Inspect reputation",
    ),
];

const CHARACTER_DELETED: &[Command] = &[
    stub("list", &[], Security::Admin, "List deleted characters"),
    stub(
        "restore",
        &[],
        Security::Admin,
        "Restore a deleted character",
    ),
    stub("delete", &[], Security::Admin, "Purge a deleted character"),
    stub("old", &[], Security::Admin, "Purge old deleted characters"),
];

const RESET: &[Command] = &[
    stub("talents", &[], Security::Admin, "Reset talents"),
    stub("spells", &[], Security::Admin, "Reset spells"),
    stub("stats", &[], Security::Admin, "Reset stats"),
    stub("honor", &[], Security::Admin, "Reset honor"),
    stub("level", &[], Security::Admin, "Reset level"),
];

const CHEAT: &[Command] = &[
    stub("god", &[], Security::GameMaster, "God mode"),
    stub("cooldown", &[], Security::GameMaster, "No cooldowns"),
    stub("casttime", &[], Security::GameMaster, "Instant casts"),
    stub("power", &[], Security::GameMaster, "No power cost"),
    stub("waterwalk", &[], Security::GameMaster, "Walk on water"),
    stub("taxi", &[], Security::GameMaster, "All flight paths"),
    stub("explore", &[], Security::GameMaster, "Explore the map"),
    stub("status", &[], Security::GameMaster, "List active cheats"),
];

const SERVER: &[Command] = &[
    live(
        "info",
        &[],
        Security::Player,
        "Server version and players",
        handlers::server_info,
    ),
    live(
        "motd",
        &[],
        Security::Player,
        "Show the message of the day",
        handlers::server_motd,
    ),
    nested(
        "set",
        &[],
        Security::Admin,
        "Set server values",
        SERVER_SET,
        None,
    ),
    stub("shutdown", &[], Security::Admin, "Timed shutdown"),
    stub("restart", &[], Security::Admin, "Timed restart"),
    stub("exit", &[], Security::Admin, "Immediate process exit"),
    stub("plimit", &[], Security::Admin, "Player login cap"),
    stub("corpses", &[], Security::GameMaster, "Expire corpses"),
    stub("idleshutdown", &[], Security::Admin, "Shutdown when empty"),
    stub("idlerestart", &[], Security::Admin, "Restart when empty"),
];

const SERVER_SET: &[Command] = &[
    live(
        "motd",
        &[],
        Security::Admin,
        "Set the message of the day",
        handlers::server_set_motd,
    ),
    stub("closed", &[], Security::Admin, "Block new logins"),
    stub("loglevel", &[], Security::Admin, "Set log level"),
];

const SEND: &[Command] = &[
    stub("mail", &[], Security::Moderator, "Send mail"),
    stub("items", &[], Security::Admin, "Mail items"),
    stub("money", &[], Security::Admin, "Mail gold"),
    stub(
        "message",
        &[],
        Security::Admin,
        "Screen message to a player",
    ),
];

const RELOAD: &[Command] = &[
    stub("all", &[], Security::Admin, "Reload all tables"),
    stub("config", &[], Security::Admin, "Reload config"),
];

const BAN: &[Command] = &[
    stub("account", &[], Security::Admin, "Ban an account"),
    stub("character", &[], Security::Admin, "Ban a character"),
    stub("ip", &[], Security::Admin, "Ban an IP"),
];

const UNBAN: &[Command] = &[
    stub("account", &[], Security::Admin, "Unban an account"),
    stub("character", &[], Security::Admin, "Unban a character"),
    stub("ip", &[], Security::Admin, "Unban an IP"),
];

const BANINFO: &[Command] = &[
    stub("account", &[], Security::Admin, "Account ban info"),
    stub("character", &[], Security::Admin, "Character ban info"),
    stub("ip", &[], Security::Admin, "IP ban info"),
];

const BANLIST: &[Command] = &[
    stub("account", &[], Security::Admin, "List banned accounts"),
    stub("character", &[], Security::Admin, "List banned characters"),
    stub("ip", &[], Security::Admin, "List banned IPs"),
];

const TICKET: &[Command] = &[
    stub("list", &[], Security::Moderator, "Open tickets"),
    stub(
        "onlinelist",
        &[],
        Security::Moderator,
        "Open tickets (owner online)",
    ),
    stub("closedlist", &[], Security::Moderator, "Closed tickets"),
    stub("viewid", &[], Security::Moderator, "View ticket by id"),
    stub(
        "viewname",
        &[],
        Security::Moderator,
        "View ticket by creator",
    ),
    stub("comment", &[], Security::Moderator, "Internal ticket note"),
    stub("assign", &[], Security::Moderator, "Assign a ticket"),
    stub("unassign", &[], Security::Moderator, "Unassign a ticket"),
    stub("close", &[], Security::Moderator, "Close a ticket"),
    stub("delete", &[], Security::Admin, "Delete a closed ticket"),
];

const DEBUG: &[Command] = &[
    nested(
        "play",
        &[],
        Security::Moderator,
        "Client playback",
        DEBUG_PLAY,
        None,
    ),
    stub(
        "areatriggers",
        &[],
        Security::Moderator,
        "Notify on areatrigger",
    ),
];

const DEBUG_PLAY: &[Command] = &[
    stub("sound", &[], Security::Moderator, "Play a sound"),
    stub("cinematic", &[], Security::Moderator, "Play a cinematic"),
];

const INSTANCE: &[Command] = &[
    stub("listbinds", &[], Security::Admin, "List instance binds"),
    stub("unbind", &[], Security::Admin, "Unbind instances"),
    stub("stats", &[], Security::Admin, "Instance stats"),
    stub("savedata", &[], Security::Admin, "Save instance data"),
];

const HONOR: &[Command] = &[
    stub("add", &[], Security::GameMaster, "Add honor"),
    stub("addkill", &[], Security::GameMaster, "Count a PvP kill"),
    stub("update", &[], Security::GameMaster, "Roll honor"),
];

const GUILD: &[Command] = &[
    stub("create", &[], Security::GameMaster, "Create a guild"),
    stub("delete", &[], Security::Admin, "Delete a guild"),
    stub("invite", &[], Security::GameMaster, "Invite to a guild"),
    stub("uninvite", &[], Security::GameMaster, "Remove from a guild"),
    stub("rank", &[], Security::GameMaster, "Set guild rank"),
];

const GROUP: &[Command] = &[
    stub("leader", &[], Security::Admin, "Set group leader"),
    stub("remove", &[], Security::Admin, "Remove from group"),
    stub("disband", &[], Security::Admin, "Disband group"),
];

const PET: &[Command] = &[
    stub("create", &[], Security::GameMaster, "Create a pet"),
    stub("learn", &[], Security::GameMaster, "Teach a pet spell"),
    stub("unlearn", &[], Security::GameMaster, "Unlearn a pet spell"),
];

const WP: &[Command] = &[
    stub("add", &[], Security::GameMaster, "Add a waypoint"),
    stub("load", &[], Security::GameMaster, "Load waypoints"),
    stub("unload", &[], Security::GameMaster, "Unload waypoints"),
    stub("show", &[], Security::GameMaster, "Show waypoints"),
    stub("reload", &[], Security::Admin, "Reload waypoints"),
    stub("modify", &[], Security::GameMaster, "Modify a waypoint"),
    stub("event", &[], Security::GameMaster, "Waypoint event"),
];

const EVENT: &[Command] = &[
    stub("list", &["activelist"], Security::GameMaster, "List events"),
    stub("start", &[], Security::GameMaster, "Start an event"),
    stub("stop", &[], Security::GameMaster, "Stop an event"),
];

const DISABLE: &[Command] = &[
    stub("add", &[], Security::Admin, "Disable a feature"),
    stub("remove", &[], Security::Admin, "Re-enable a feature"),
];

const PDUMP: &[Command] = &[
    stub("write", &[], Security::Admin, "Write a character dump"),
    stub("load", &[], Security::Admin, "Load a character dump"),
];
