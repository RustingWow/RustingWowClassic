use wow_shared::{
    Appearance, CharacterClass, CharacterGender, CharacterRace, CharacterTemplate, CreatureFaction,
    Position,
};

use crate::appearance::{HUMAN_MALE_DISPLAY_ID, display_id, faction};

pub const PLAYER_MAX_HEALTH: i32 = 100;
pub const PLAYER_DAMAGE: i32 = 12;
pub const STAND_STATE_STAND: u8 = 0;
pub const STAND_STATE_SIT: u8 = 1;
pub const STAND_STATE_DEAD: u8 = 7;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub guid: u64,
    pub name: String,
    pub position: Position,
    pub health: i32,
    pub max_health: i32,
    pub stand_state: u8,
    pub race: CharacterRace,
    pub class: CharacterClass,
    pub gender: CharacterGender,
    pub appearance: Appearance,
    pub display_id: i32,
    pub faction: i32,
    #[serde(default)]
    pub copper: u32,
    #[serde(default)]
    pub bag: [Option<(u32, u32)>; 16],
    #[serde(default)]
    pub gm_on: bool,
    #[serde(default = "default_true")]
    pub gm_visible: bool,
    #[serde(default)]
    pub gm_chat: bool,
    #[serde(default)]
    pub gmlevel: u8,
    #[serde(default)]
    pub quest_log: Vec<QuestLogEntry>,
    #[serde(default)]
    pub rewarded_quests: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct QuestLogEntry {
    pub quest_id: u32,
    pub kills: [u32; 4],
    pub complete: bool,
}

pub const QUEST_LOG_SLOTS: usize = 20;

fn default_true() -> bool {
    true
}

impl Player {
    pub fn new(guid: u64, name: impl Into<String>, position: Position) -> Self {
        Self {
            guid,
            name: name.into(),
            position,
            health: PLAYER_MAX_HEALTH,
            max_health: PLAYER_MAX_HEALTH,
            stand_state: STAND_STATE_STAND,
            race: CharacterRace::Human,
            class: CharacterClass::Warrior,
            gender: CharacterGender::Male,
            appearance: Appearance::default(),
            display_id: HUMAN_MALE_DISPLAY_ID,
            faction: CreatureFaction::PlayerHuman.as_protocol() as i32,
            copper: 10_000,
            bag: [None; 16],
            gm_on: false,
            gm_visible: true,
            gm_chat: false,
            gmlevel: 0,
            quest_log: Vec::new(),
            rewarded_quests: Vec::new(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn add_item(&mut self, item_id: u32, count: u32, stackable: u32) -> bool {
        let stackable = stackable.max(1);
        let mut remaining = count;
        for slot in &mut self.bag {
            if remaining == 0 {
                break;
            }
            if let Some((existing, stacked)) = slot {
                if *existing == item_id && *stacked < stackable {
                    let room = stackable - *stacked;
                    let add = remaining.min(room);
                    *stacked += add;
                    remaining -= add;
                }
            }
        }
        for slot in &mut self.bag {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let add = remaining.min(stackable);
                *slot = Some((item_id, add));
                remaining -= add;
            }
        }
        remaining == 0
    }

    pub fn item_count(&self, item_id: u32) -> u32 {
        self.bag
            .iter()
            .filter_map(|slot| *slot)
            .filter(|(id, _)| *id == item_id)
            .map(|(_, count)| count)
            .sum()
    }
}

impl From<&CharacterTemplate> for Player {
    fn from(character: &CharacterTemplate) -> Self {
        let mut player = Self::new(character.guid, character.name.clone(), character.position);
        player.race = character.race;
        player.class = character.class;
        player.gender = character.gender;
        player.appearance = character.appearance;
        player.display_id = display_id(character.race, character.gender);
        player.faction = faction(character.race).as_protocol() as i32;
        player
    }
}
