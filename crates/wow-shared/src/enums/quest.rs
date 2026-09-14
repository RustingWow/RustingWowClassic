use serde::{Deserialize, Serialize};

use super::db_enum;

/// Vanilla 1.12 `QuestInfo.dbc` id (`quest_template.Type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QuestType {
    #[default]
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "ELITE")]
    Elite,
    #[serde(rename = "LIFE")]
    Life,
    #[serde(rename = "PVP")]
    Pvp,
    #[serde(rename = "RAID")]
    Raid,
    #[serde(rename = "DUNGEON")]
    Dungeon,
    #[serde(rename = "WORLD_EVENT")]
    WorldEvent,
    #[serde(rename = "LEGENDARY")]
    Legendary,
    #[serde(rename = "ESCORT")]
    Escort,
}

db_enum!(
    QuestType,
    u32,
    None => 0, "NONE",
    Elite => 1, "ELITE",
    Life => 21, "LIFE",
    Pvp => 41, "PVP",
    Raid => 62, "RAID",
    Dungeon => 81, "DUNGEON",
    WorldEvent => 82, "WORLD_EVENT",
    Legendary => 83, "LEGENDARY",
    Escort => 84, "ESCORT",
);
