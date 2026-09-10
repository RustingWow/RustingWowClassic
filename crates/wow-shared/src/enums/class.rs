use serde::{Deserialize, Serialize};

use super::db_enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterClass {
    Warrior,
    Paladin,
    Hunter,
    Rogue,
    Priest,
    Shaman,
    Mage,
    Warlock,
    Druid,
}

db_enum!(
    CharacterClass,
    u8,
    Warrior => 1, "WARRIOR",
    Paladin => 2, "PALADIN",
    Hunter => 3, "HUNTER",
    Rogue => 4, "ROGUE",
    Priest => 5, "PRIEST",
    Shaman => 7, "SHAMAN",
    Mage => 8, "MAGE",
    Warlock => 9, "WARLOCK",
    Druid => 11, "DRUID",
);
