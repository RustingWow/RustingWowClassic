use serde::{Deserialize, Serialize};

macro_rules! db_enum {
    ($name:ident, $proto:ty, $($variant:ident => $protocol:expr, $db:expr),+ $(,)?) => {
        impl $name {
            pub const fn as_protocol(self) -> $proto {
                match self {
                    $(Self::$variant => $protocol,)+
                }
            }

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $db,)+
                }
            }

            pub fn from_protocol(value: $proto) -> Option<Self> {
                match value {
                    $($protocol => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub fn from_str(value: &str) -> Option<Self> {
                match value {
                    $($db => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterGender {
    Male,
    Female,
}

db_enum!(
    CharacterGender,
    u8,
    Male => 0, "MALE",
    Female => 1, "FEMALE",
);

/// Vanilla 1.12 area (zone/subzone). Same type the client uses on the wire.
pub type CharacterArea = wow_world_base::vanilla::Area;
/// Vanilla 1.12 continent/instance map. Same type the client uses on the wire.
pub type CharacterMap = wow_world_base::vanilla::Map;

/// Protocol id + uppercase DB label for Vanilla map/area enums.
pub trait DbEnum: Copy + Sized + 'static {
    fn as_protocol(self) -> u32;
    fn as_str(self) -> &'static str;
    fn from_protocol(value: u32) -> Option<Self>;
    fn from_str(value: &str) -> Option<Self>;
}

impl DbEnum for CharacterArea {
    fn as_protocol(self) -> u32 {
        self.as_int()
    }

    fn as_str(self) -> &'static str {
        self.as_test_case_value()
    }

    fn from_protocol(value: u32) -> Option<Self> {
        Self::from_int(value).ok()
    }

    fn from_str(value: &str) -> Option<Self> {
        area_by_db().get(value).copied()
    }
}

impl DbEnum for CharacterMap {
    fn as_protocol(self) -> u32 {
        self.as_int()
    }

    fn as_str(self) -> &'static str {
        self.as_test_case_value()
    }

    fn from_protocol(value: u32) -> Option<Self> {
        Self::from_int(value).ok()
    }

    fn from_str(value: &str) -> Option<Self> {
        map_by_db().get(value).copied()
    }
}

fn area_by_db() -> &'static std::collections::HashMap<&'static str, CharacterArea> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, CharacterArea>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        CharacterArea::variants()
            .into_iter()
            .map(|area| (area.as_test_case_value(), area))
            .collect()
    })
}

fn map_by_db() -> &'static std::collections::HashMap<&'static str, CharacterMap> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, CharacterMap>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        CharacterMap::variants()
            .into_iter()
            .map(|map| (map.as_test_case_value(), map))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_elf_hunter_matches_protocol_and_db() {
        assert_eq!(
            CharacterRace::from_protocol(4),
            Some(CharacterRace::NightElf)
        );
        assert_eq!(
            CharacterClass::from_protocol(3),
            Some(CharacterClass::Hunter)
        );
        assert_eq!(
            CharacterGender::from_protocol(0),
            Some(CharacterGender::Male)
        );
        assert_eq!(
            CharacterArea::from_protocol(188),
            Some(CharacterArea::Shadowglen)
        );
        assert_eq!(CharacterRace::NightElf.as_str(), "NIGHT_ELF");
        assert_eq!(CharacterClass::Hunter.as_str(), "HUNTER");
        assert_eq!(CharacterGender::Male.as_str(), "MALE");
        assert_eq!(CharacterArea::Shadowglen.as_str(), "SHADOWGLEN");
        assert_eq!(CharacterMap::from_protocol(1), Some(CharacterMap::Kalimdor));
        assert_eq!(CharacterMap::Kalimdor.as_str(), "KALIMDOR");
        assert_eq!(
            CharacterMap::from_protocol(0),
            Some(CharacterMap::EasternKingdoms)
        );
        assert_eq!(
            CharacterMap::from_protocol(389),
            Some(CharacterMap::RagefireChasm)
        );
        assert_eq!(CharacterMap::RagefireChasm.as_str(), "RAGEFIRE_CHASM");
        assert_eq!(
            CharacterArea::from_protocol(12),
            Some(CharacterArea::ElwynnForest)
        );
        assert_eq!(CharacterArea::ElwynnForest.as_str(), "ELWYNN_FOREST");
        assert_eq!(
            CharacterArea::from_str("UNUSED_THE_DEADMINES_002"),
            Some(CharacterArea::UnusedTheDeadmines002)
        );
    }
}
