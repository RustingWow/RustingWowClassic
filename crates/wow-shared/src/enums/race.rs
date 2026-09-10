use serde::{Deserialize, Serialize};

use super::db_enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterRace {
    Human,
    Orc,
    Dwarf,
    NightElf,
    Undead,
    Tauren,
    Gnome,
    Troll,
}

db_enum!(
    CharacterRace,
    u8,
    Human => 1, "HUMAN",
    Orc => 2, "ORC",
    Dwarf => 3, "DWARF",
    NightElf => 4, "NIGHT_ELF",
    Undead => 5, "UNDEAD",
    Tauren => 6, "TAUREN",
    Gnome => 7, "GNOME",
    Troll => 8, "TROLL",
);
