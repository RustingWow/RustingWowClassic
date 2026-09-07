use wow_shared::{CharacterTemplate, Position};

pub const PLAYER_MAX_HEALTH: i32 = 100;
pub const PLAYER_DAMAGE: i32 = 12;
pub const STAND_STATE_STAND: u8 = 0;
pub const STAND_STATE_DEAD: u8 = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub guid: u64,
    pub name: String,
    pub position: Position,
    pub health: i32,
    pub max_health: i32,
    pub stand_state: u8,
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
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

impl From<&CharacterTemplate> for Player {
    fn from(character: &CharacterTemplate) -> Self {
        Self::new(character.guid, character.name.clone(), character.position)
    }
}
