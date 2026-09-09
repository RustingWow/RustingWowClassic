use wow_shared::{
    CharacterArea, CharacterMap, CharacterRace, NORTHSHIRE_ORIENTATION, NORTHSHIRE_X, NORTHSHIRE_Y,
    NORTHSHIRE_Z, Position,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceStart {
    pub map_id: CharacterMap,
    pub position: Position,
    pub area: CharacterArea,
}

const HUMAN: RaceStart = RaceStart {
    map_id: CharacterMap::EasternKingdoms,
    position: Position {
        x: NORTHSHIRE_X,
        y: NORTHSHIRE_Y,
        z: NORTHSHIRE_Z,
        orientation: NORTHSHIRE_ORIENTATION,
    },
    area: CharacterArea::NorthshireValley,
};

const ORC: RaceStart = RaceStart {
    map_id: CharacterMap::Kalimdor,
    position: Position {
        x: -618.518,
        y: -4251.67,
        z: 38.718,
        orientation: 0.0,
    },
    area: CharacterArea::ValleyOfTrials,
};

const DWARF: RaceStart = RaceStart {
    map_id: CharacterMap::EasternKingdoms,
    position: Position {
        x: -6240.32,
        y: 331.033,
        z: 382.758,
        orientation: 6.17716,
    },
    area: CharacterArea::ColdridgeValley,
};

const NIGHT_ELF: RaceStart = RaceStart {
    map_id: CharacterMap::Kalimdor,
    position: Position {
        x: 10311.3,
        y: 832.463,
        z: 1326.41,
        orientation: 5.69632,
    },
    area: CharacterArea::Shadowglen,
};

const UNDEAD: RaceStart = RaceStart {
    map_id: CharacterMap::EasternKingdoms,
    position: Position {
        x: 1676.71,
        y: 1678.31,
        z: 121.67,
        orientation: 2.70526,
    },
    area: CharacterArea::Deathknell,
};

const TAUREN: RaceStart = RaceStart {
    map_id: CharacterMap::Kalimdor,
    position: Position {
        x: -2917.58,
        y: -257.98,
        z: 52.9968,
        orientation: 0.0,
    },
    area: CharacterArea::CampNarache,
};

pub fn race_start(race: CharacterRace) -> RaceStart {
    match race {
        CharacterRace::Human => HUMAN,
        CharacterRace::Orc | CharacterRace::Troll => ORC,
        CharacterRace::Dwarf | CharacterRace::Gnome => DWARF,
        CharacterRace::NightElf => NIGHT_ELF,
        CharacterRace::Undead => UNDEAD,
        CharacterRace::Tauren => TAUREN,
    }
}
