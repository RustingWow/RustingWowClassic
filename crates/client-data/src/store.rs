use std::path::Path;

use wow_dbc::DbcTable;
use wow_dbc::vanilla_tables::faction_template::FactionTemplate;

use crate::faction::FactionTemplates;

/// Client DBC tables loaded from a directory produced by `extract-dbc`.
#[derive(Clone, Debug)]
pub struct DbcStore {
    pub factions: FactionTemplates,
}

impl DbcStore {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join(FactionTemplate::FILENAME);
        Ok(Self {
            factions: FactionTemplates::load(&path)?,
        })
    }

    pub fn load_or_builtin(dir: &Path) -> Self {
        Self {
            factions: FactionTemplates::load_or_builtin(dir),
        }
    }
}
