use wow_shared::{
    Appearance, CharacterClass, CharacterGender, CharacterRace, CharacterTemplate, Position,
};

use crate::appearance::{HUMAN_MALE_DISPLAY_ID, display_id, faction};

pub const PLAYER_MAX_HEALTH: i32 = 100;
pub const PLAYER_DAMAGE: i32 = 12;
pub const STAND_STATE_STAND: u8 = 0;
pub const STAND_STATE_DEAD: u8 = 7;

const FACTION_HUMAN: i32 = 1;

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
            faction: FACTION_HUMAN,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
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
        player.faction = faction(character.race);
        player
    }
}
