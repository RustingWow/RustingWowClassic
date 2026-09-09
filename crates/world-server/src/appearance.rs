use wow_shared::{CharacterClass, CharacterGender, CharacterRace};
use wow_world_messages::vanilla::{Class, Gender, Power, Race, RaceClass};

pub const HUMAN_MALE_DISPLAY_ID: i32 = 49;

const FACTION_HUMAN: i32 = 1;
const FACTION_ORC: i32 = 2;
const FACTION_DWARF: i32 = 3;
const FACTION_NIGHT_ELF: i32 = 4;
const FACTION_UNDEAD: i32 = 5;
const FACTION_TAUREN: i32 = 6;
const FACTION_GNOME: i32 = 115;
const FACTION_TROLL: i32 = 116;

pub fn race_allowed(race: CharacterRace, class: CharacterClass) -> bool {
    RaceClass::try_from((self::race(race), self::class(class))).is_ok()
}

pub fn display_id(race: CharacterRace, gender: CharacterGender) -> i32 {
    let female = gender == CharacterGender::Female;
    match race {
        CharacterRace::Human => {
            if female {
                50
            } else {
                49
            }
        }
        CharacterRace::Orc => {
            if female {
                52
            } else {
                51
            }
        }
        CharacterRace::Dwarf => {
            if female {
                54
            } else {
                53
            }
        }
        CharacterRace::NightElf => {
            if female {
                56
            } else {
                55
            }
        }
        CharacterRace::Undead => {
            if female {
                58
            } else {
                57
            }
        }
        CharacterRace::Tauren => {
            if female {
                60
            } else {
                59
            }
        }
        CharacterRace::Gnome => {
            if female {
                1564
            } else {
                1563
            }
        }
        CharacterRace::Troll => {
            if female {
                1479
            } else {
                1478
            }
        }
    }
}

pub fn faction(race: CharacterRace) -> i32 {
    match race {
        CharacterRace::Human => FACTION_HUMAN,
        CharacterRace::Orc => FACTION_ORC,
        CharacterRace::Dwarf => FACTION_DWARF,
        CharacterRace::NightElf => FACTION_NIGHT_ELF,
        CharacterRace::Undead => FACTION_UNDEAD,
        CharacterRace::Tauren => FACTION_TAUREN,
        CharacterRace::Gnome => FACTION_GNOME,
        CharacterRace::Troll => FACTION_TROLL,
    }
}

pub fn race(value: CharacterRace) -> Race {
    match value {
        CharacterRace::Human => Race::Human,
        CharacterRace::Orc => Race::Orc,
        CharacterRace::Dwarf => Race::Dwarf,
        CharacterRace::NightElf => Race::NightElf,
        CharacterRace::Undead => Race::Undead,
        CharacterRace::Tauren => Race::Tauren,
        CharacterRace::Gnome => Race::Gnome,
        CharacterRace::Troll => Race::Troll,
    }
}

pub fn class(value: CharacterClass) -> Class {
    match value {
        CharacterClass::Warrior => Class::Warrior,
        CharacterClass::Paladin => Class::Paladin,
        CharacterClass::Hunter => Class::Hunter,
        CharacterClass::Rogue => Class::Rogue,
        CharacterClass::Priest => Class::Priest,
        CharacterClass::Shaman => Class::Shaman,
        CharacterClass::Mage => Class::Mage,
        CharacterClass::Warlock => Class::Warlock,
        CharacterClass::Druid => Class::Druid,
    }
}

pub fn gender(value: CharacterGender) -> Gender {
    match value {
        CharacterGender::Male => Gender::Male,
        CharacterGender::Female => Gender::Female,
    }
}

pub fn power(class: CharacterClass) -> Power {
    match class {
        CharacterClass::Warrior => Power::Rage,
        CharacterClass::Rogue => Power::Energy,
        _ => Power::Mana,
    }
}

pub fn normalize_character_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if !(2..=12).contains(&trimmed.len()) || !trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(trimmed.to_string())
}
