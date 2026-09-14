use std::path::{Path, PathBuf};

use wow_dbc::DbcTable;
use wow_dbc::vanilla_tables::faction_template::FactionTemplate;

use crate::faction_masks::MASKS;

const PLAYER_GROUP: u32 = 0x1;
const ALLIANCE_GROUP: u32 = 0x2;
const HORDE_GROUP: u32 = 0x4;
const PLAYER_FACTION_GROUPS: u32 = PLAYER_GROUP | ALLIANCE_GROUP | HORDE_GROUP;
const MONSTER_GROUP: u32 = 0x8;
const BEAST_WOLF: i32 = 32;
const BEAST_WOLF_HOSTILE: i32 = 38;
const MONSTER_TEMPLATE: i32 = 14;

/// Vanilla `FactionTemplate.dbc` hostility masks (`ourMask` / `hostileMask`).
#[derive(Clone, Debug)]
pub struct FactionTemplates {
    source: Source,
}

#[derive(Clone, Debug)]
enum Source {
    Builtin,
    Loaded {
        path: PathBuf,
        masks: Vec<(i32, u32, u32)>,
    },
}

impl FactionTemplates {
    pub fn builtin() -> Self {
        Self {
            source: Source::Builtin,
        }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        let table = FactionTemplate::read(&mut bytes.as_slice())?;
        let mut masks: Vec<(i32, u32, u32)> = table
            .rows
            .iter()
            .map(|row| (row.id.id as i32, row.faction_group.id, row.enemy_group.id))
            .collect();
        masks.sort_by_key(|row| row.0);
        Ok(Self {
            source: Source::Loaded {
                path: path.to_path_buf(),
                masks,
            },
        })
    }

    pub fn load_or_builtin(dir: &Path) -> Self {
        let path = dir.join(FactionTemplate::FILENAME);
        match Self::load(&path) {
            Ok(loaded) => {
                tracing::info!(
                    count = loaded.len(),
                    path = %path.display(),
                    "faction templates from DBC"
                );
                loaded
            }
            Err(_) if !path.exists() => {
                tracing::warn!(
                    path = %path.display(),
                    "using builtin FactionTemplate masks"
                );
                Self::builtin()
            }
            Err(error) => {
                tracing::warn!(
                    %error,
                    path = %path.display(),
                    "failed to load FactionTemplate.dbc; using builtin masks"
                );
                Self::builtin()
            }
        }
    }

    pub fn len(&self) -> usize {
        match &self.source {
            Source::Builtin => MASKS.len(),
            Source::Loaded { masks, .. } => masks.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_builtin(&self) -> bool {
        matches!(self.source, Source::Builtin)
    }

    pub fn source_path(&self) -> Option<&Path> {
        match &self.source {
            Source::Builtin => None,
            Source::Loaded { path, .. } => Some(path),
        }
    }

    /// True when the 1.12 client treats this FactionTemplate as hostile to players
    /// (`ourMask` includes Monster, or `hostileMask` includes Player).
    pub fn hostile_to_players(&self, template: i32) -> bool {
        match self.masks(template) {
            Some((our_mask, hostile_mask)) => {
                (our_mask & MONSTER_GROUP) != 0 || (hostile_mask & PLAYER_GROUP) != 0
            }
            None => false,
        }
    }

    /// Alliance / Horde / player-group templates (Stormwind, Darnassus, …).
    /// Starting-zone wildlife uses Creature 7 / 189 with empty masks — not this.
    pub fn player_faction_aligned(&self, template: i32) -> bool {
        match self.masks(template) {
            Some((our_mask, _)) => {
                (our_mask & PLAYER_FACTION_GROUPS) != 0 && (our_mask & MONSTER_GROUP) == 0
            }
            None => false,
        }
    }

    /// Combat NPCs that are not city-aligned get a client-hostile template:
    /// already-hostile stays, wolf 32 becomes 38, other neutrals become Monster 14.
    /// City factions stay unchanged so the client keeps a friendly cursor.
    pub fn ensure_hostile_to_players(&self, template: i32) -> i32 {
        if self.hostile_to_players(template) || self.player_faction_aligned(template) {
            template
        } else if template == BEAST_WOLF {
            BEAST_WOLF_HOSTILE
        } else {
            MONSTER_TEMPLATE
        }
    }

    fn masks(&self, template: i32) -> Option<(u32, u32)> {
        let rows = match &self.source {
            Source::Builtin => MASKS,
            Source::Loaded { masks, .. } => masks.as_slice(),
        };
        rows.binary_search_by_key(&template, |row| row.0)
            .ok()
            .map(|index| (rows[index].1, rows[index].2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_dbc::vanilla_tables::faction::FactionKey;
    use wow_dbc::vanilla_tables::faction_group::FactionGroupKey;
    use wow_dbc::vanilla_tables::faction_template::{
        FactionTemplate, FactionTemplateKey, FactionTemplateRow,
    };

    #[test]
    fn builtin_wolf_and_stormwind() {
        let factions = FactionTemplates::builtin();
        assert!(!factions.hostile_to_players(32));
        assert_eq!(factions.ensure_hostile_to_players(32), 38);
        assert!(factions.hostile_to_players(14));
        assert!(factions.hostile_to_players(38));
        assert!(!factions.hostile_to_players(11));
        assert!(factions.player_faction_aligned(11));
        assert_eq!(factions.ensure_hostile_to_players(11), 11);
        assert!(factions.player_faction_aligned(80));
        assert_eq!(factions.ensure_hostile_to_players(80), 80);
        assert!(!factions.hostile_to_players(7));
        assert!(!factions.player_faction_aligned(7));
        assert_eq!(factions.ensure_hostile_to_players(7), 14);
        assert_eq!(factions.ensure_hostile_to_players(189), 14);
        assert!(!factions.hostile_to_players(9999));
        assert_eq!(factions.ensure_hostile_to_players(9999), 14);
    }

    #[test]
    fn load_synthetic_faction_template_dbc() {
        let table = FactionTemplate {
            rows: vec![row(11, 3, 12), row(32, 0, 0), row(38, 8, 1)],
        };
        let dir =
            std::env::temp_dir().join(format!("wowserver-client-data-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("FactionTemplate.dbc");
        let mut bytes = Vec::new();
        table.write(&mut bytes).unwrap();
        std::fs::write(&path, &bytes).unwrap();

        let loaded = FactionTemplates::load(&path).unwrap();
        assert!(!loaded.is_builtin());
        assert_eq!(loaded.len(), 3);
        assert!(!loaded.hostile_to_players(32));
        assert_eq!(loaded.ensure_hostile_to_players(32), 38);
        assert!(loaded.hostile_to_players(38));
        assert!(!loaded.hostile_to_players(11));

        let from_dir = FactionTemplates::load_or_builtin(&dir);
        assert!(!from_dir.is_builtin());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn row(id: u32, our_mask: u32, hostile_mask: u32) -> FactionTemplateRow {
        FactionTemplateRow {
            id: FactionTemplateKey::new(id),
            faction: FactionKey::new(0),
            flags: Default::default(),
            faction_group: FactionGroupKey::new(our_mask),
            friend_group: FactionGroupKey::new(0),
            enemy_group: FactionGroupKey::new(hostile_mask),
            enemies: [0; 4],
            friends: [0; 4],
        }
    }
}
