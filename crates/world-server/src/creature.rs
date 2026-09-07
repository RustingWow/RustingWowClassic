use wow_shared::Position;
use wow_world_messages::vanilla::CreatureFamily;

const UNIT_HIGH: u64 = 0xF130;

pub const ENTRY_NORTHSHIRE_GUARD: u32 = 1423;
pub const ENTRY_YOUNG_WOLF: u32 = 299;
pub const DISPLAY_NORTHSHIRE_GUARD: i32 = 3167;
pub const DISPLAY_YOUNG_WOLF: i32 = 447;
pub const FACTION_STORMWIND: i32 = 11;
pub const FACTION_MONSTER: i32 = 14;
pub const NPC_FLAG_GOSSIP: i32 = 1;
pub const GUARD_GOSSIP_TEXT_ID: u32 = 900_001;
pub const GUARD_GOSSIP_TEXT: &str =
    "Welcome to Northshire, recruit. Keep your weapon close—wolves still hunt these woods.";
pub const GUARD_GOSSIP_WOLF_TEXT_ID: u32 = 900_002;
pub const GUARD_GOSSIP_WOLF_TEXT: &str =
    "A young wolf prowls the courtyard ahead. Thin it out if you want to prove yourself.";
pub const GUARD_GOSSIP_ASK_ID: u32 = 0;
pub const GUARD_GOSSIP_ACCEPT_ID: u32 = 1;
pub const CREATURE_TYPE_BEAST: u32 = 1;
pub const CREATURE_TYPE_HUMANOID: u32 = 7;
pub const WOLF_HEALTH: i32 = 45;
pub const WOLF_DAMAGE: i32 = 4;
pub const GUARD_HEALTH: i32 = 100;

#[derive(Clone, Debug, PartialEq)]
pub struct Creature {
    pub guid: u64,
    pub entry: u32,
    pub name: String,
    pub sub_name: String,
    pub display_id: i32,
    pub faction: i32,
    pub position: Position,
    pub health: i32,
    pub max_health: i32,
    pub level: i32,
    pub npc_flags: i32,
    pub creature_type: u32,
    pub family: CreatureFamily,
    pub civilian: bool,
    pub hostile: bool,
    pub dead: bool,
    pub gossip: Option<Gossip>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gossip {
    pub menus: Vec<GossipMenu>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GossipMenu {
    pub text_id: u32,
    pub text: String,
    pub options: Vec<GossipOption>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GossipOption {
    pub id: u32,
    pub text: String,
    pub action: GossipAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GossipAction {
    Close,
    ShowMenu { text_id: u32 },
}

impl Gossip {
    pub fn greeting(&self) -> &GossipMenu {
        &self.menus[0]
    }

    pub fn menu(&self, text_id: u32) -> Option<&GossipMenu> {
        self.menus.iter().find(|menu| menu.text_id == text_id)
    }

    pub fn option(&self, option_id: u32) -> Option<&GossipOption> {
        self.menus
            .iter()
            .flat_map(|menu| menu.options.iter())
            .find(|option| option.id == option_id)
    }
}

impl Creature {
    pub fn can_gossip(&self) -> bool {
        self.npc_flags & NPC_FLAG_GOSSIP != 0
            && !self.dead
            && !self.hostile
            && self.gossip.is_some()
    }
}

pub fn unit_guid(counter: u32, entry: u32) -> u64 {
    u64::from(counter) | (u64::from(entry) << 24) | (UNIT_HIGH << 48)
}

pub fn is_creature_guid(guid: u64) -> bool {
    guid >> 48 == UNIT_HIGH
}

pub fn northshire_guard_guid() -> u64 {
    unit_guid(1, ENTRY_NORTHSHIRE_GUARD)
}

pub fn northshire_wolf_guid() -> u64 {
    unit_guid(2, ENTRY_YOUNG_WOLF)
}

pub fn northshire_guard() -> Creature {
    Creature {
        guid: northshire_guard_guid(),
        entry: ENTRY_NORTHSHIRE_GUARD,
        name: "Northshire Guard".to_string(),
        sub_name: String::new(),
        display_id: DISPLAY_NORTHSHIRE_GUARD,
        faction: FACTION_STORMWIND,
        position: Position {
            x: Position::NORTHSHIRE.x + 6.0,
            y: Position::NORTHSHIRE.y + 2.0,
            z: Position::NORTHSHIRE.z,
            orientation: std::f32::consts::PI,
        },
        health: GUARD_HEALTH,
        max_health: GUARD_HEALTH,
        level: 5,
        npc_flags: NPC_FLAG_GOSSIP,
        creature_type: CREATURE_TYPE_HUMANOID,
        family: CreatureFamily::None,
        civilian: false,
        hostile: false,
        dead: false,
        gossip: Some(northshire_guard_gossip()),
    }
}

fn northshire_guard_gossip() -> Gossip {
    Gossip {
        menus: vec![
            GossipMenu {
                text_id: GUARD_GOSSIP_TEXT_ID,
                text: GUARD_GOSSIP_TEXT.to_string(),
                options: vec![GossipOption {
                    id: GUARD_GOSSIP_ASK_ID,
                    text: "What's nearby?".to_string(),
                    action: GossipAction::ShowMenu {
                        text_id: GUARD_GOSSIP_WOLF_TEXT_ID,
                    },
                }],
            },
            GossipMenu {
                text_id: GUARD_GOSSIP_WOLF_TEXT_ID,
                text: GUARD_GOSSIP_WOLF_TEXT.to_string(),
                options: vec![GossipOption {
                    id: GUARD_GOSSIP_ACCEPT_ID,
                    text: "I'll take care of it.".to_string(),
                    action: GossipAction::Close,
                }],
            },
        ],
    }
}

pub fn northshire_wolf() -> Creature {
    Creature {
        guid: northshire_wolf_guid(),
        entry: ENTRY_YOUNG_WOLF,
        name: "Young Wolf".to_string(),
        sub_name: String::new(),
        display_id: DISPLAY_YOUNG_WOLF,
        faction: FACTION_MONSTER,
        position: Position {
            x: Position::NORTHSHIRE.x + 32.0,
            y: Position::NORTHSHIRE.y - 8.0,
            z: Position::NORTHSHIRE.z,
            orientation: std::f32::consts::PI,
        },
        health: WOLF_HEALTH,
        max_health: WOLF_HEALTH,
        level: 1,
        npc_flags: 0,
        creature_type: CREATURE_TYPE_BEAST,
        family: CreatureFamily::Wolf,
        civilian: false,
        hostile: true,
        dead: false,
        gossip: None,
    }
}

pub fn northshire_npcs() -> Vec<Creature> {
    vec![northshire_guard(), northshire_wolf()]
}
