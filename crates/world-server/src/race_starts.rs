use std::collections::HashMap;

use sqlx::{PgPool, Row};
use wow_shared::{
    CharacterArea, CharacterMap, CharacterRace, DbEnum, NORTHSHIRE_ORIENTATION, NORTHSHIRE_X,
    NORTHSHIRE_Y, NORTHSHIRE_Z, Position,
};

const PLAYABLE_RACES: [CharacterRace; 8] = [
    CharacterRace::Human,
    CharacterRace::Orc,
    CharacterRace::Dwarf,
    CharacterRace::NightElf,
    CharacterRace::Undead,
    CharacterRace::Tauren,
    CharacterRace::Gnome,
    CharacterRace::Troll,
];

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

#[derive(Debug, Clone)]
pub struct RaceStarts {
    by_race: HashMap<CharacterRace, RaceStart>,
}

impl RaceStarts {
    pub fn builtin() -> Self {
        let mut by_race = HashMap::with_capacity(PLAYABLE_RACES.len());
        by_race.insert(CharacterRace::Human, HUMAN);
        by_race.insert(CharacterRace::Orc, ORC);
        by_race.insert(CharacterRace::Troll, ORC);
        by_race.insert(CharacterRace::Dwarf, DWARF);
        by_race.insert(CharacterRace::Gnome, DWARF);
        by_race.insert(CharacterRace::NightElf, NIGHT_ELF);
        by_race.insert(CharacterRace::Undead, UNDEAD);
        by_race.insert(CharacterRace::Tauren, TAUREN);
        Self { by_race }
    }

    pub async fn load(pool: &PgPool) -> anyhow::Result<Self> {
        let rows = sqlx::query(
            "SELECT race::text AS race, map_id::text AS map_id, x, y, z, orientation,
                    area::text AS area
             FROM race_start_positions",
        )
        .fetch_all(pool)
        .await?;

        let mut by_race = HashMap::with_capacity(rows.len());
        for row in rows {
            let race_label: String = row.get("race");
            let Some(race) = CharacterRace::from_str(&race_label) else {
                tracing::warn!(race = %race_label, "skipped unknown race in race_start_positions");
                continue;
            };
            let map_label: String = row.get("map_id");
            let Some(map_id) = CharacterMap::from_str(&map_label) else {
                anyhow::bail!(
                    "unknown map_id in race_start_positions for {race_label}: {map_label}"
                );
            };
            let area_label: String = row.get("area");
            let Some(area) = CharacterArea::from_str(&area_label) else {
                anyhow::bail!(
                    "unknown area in race_start_positions for {race_label}: {area_label}"
                );
            };
            by_race.insert(
                race,
                RaceStart {
                    map_id,
                    position: Position {
                        x: row.get("x"),
                        y: row.get("y"),
                        z: row.get("z"),
                        orientation: row.get("orientation"),
                    },
                    area,
                },
            );
        }

        for race in PLAYABLE_RACES {
            anyhow::ensure!(
                by_race.contains_key(&race),
                "race_start_positions missing {}",
                race.as_str()
            );
        }

        tracing::info!(races = by_race.len(), "loaded race start positions");
        Ok(Self { by_race })
    }

    pub fn get(&self, race: CharacterRace) -> anyhow::Result<RaceStart> {
        self.by_race
            .get(&race)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("missing race start for {}", race.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_covers_every_playable_race() {
        let starts = RaceStarts::builtin();
        for race in PLAYABLE_RACES {
            starts.get(race).expect("builtin race start");
        }
        assert_eq!(
            starts.get(CharacterRace::Gnome).unwrap(),
            starts.get(CharacterRace::Dwarf).unwrap()
        );
        assert_eq!(
            starts.get(CharacterRace::Troll).unwrap(),
            starts.get(CharacterRace::Orc).unwrap()
        );
        assert_eq!(
            starts.get(CharacterRace::Human).unwrap().position,
            Position::NORTHSHIRE
        );
    }
}
