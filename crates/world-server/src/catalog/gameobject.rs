use sqlx::{PgPool, Row};
use wow_shared::{CharacterMap, CreatureFaction, DbEnum, Position};

use crate::gameobject::{GameObject, gameobject_guid};

use super::Catalog;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct GameObjectTypeRow {
    pub entry: u32,
    pub name: String,
    pub object_type: i32,
    pub display_id: i32,
    pub faction: i32,
    pub flags: i32,
    pub size: f32,
    pub data: [i32; 24],
}

#[derive(Clone, Debug)]
pub struct GameObjectSpawnRow {
    pub guid: u32,
    pub entry: u32,
    #[allow(dead_code)]
    pub map_id: u32,
    pub position: Position,
    pub rotation: [f32; 4],
    #[allow(dead_code)]
    pub respawn_secs: i32,
    pub anim_progress: i32,
    pub state: i32,
}

impl Catalog {
    pub fn gameobjects_for_map(&self, map_id: u32) -> Vec<GameObject> {
        let Some(spawns) = self.gameobject_spawns.get(&map_id) else {
            return Vec::new();
        };
        spawns
            .iter()
            .filter_map(|spawn| self.gameobject_from_spawn(spawn))
            .collect()
    }

    pub(crate) fn gameobject_from_spawn(&self, spawn: &GameObjectSpawnRow) -> Option<GameObject> {
        let kind = self.gameobject_types.get(&spawn.entry)?;
        if !GameObject::is_static_spawn(kind.object_type) {
            return None;
        }
        Some(GameObject {
            guid: gameobject_guid(spawn.guid, spawn.entry),
            entry: spawn.entry,
            name: kind.name.clone(),
            display_id: kind.display_id,
            object_type: kind.object_type,
            flags: kind.flags,
            faction: kind.faction,
            state: spawn.state,
            anim_progress: spawn.anim_progress,
            scale: if kind.size > 0.0 { kind.size } else { 1.0 },
            position: spawn.position,
            rotation: spawn.rotation,
            data: kind.data,
        })
    }

    pub(crate) async fn load_gameobjects(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let type_sql = format!(
            "SELECT entry, name, object_type, display_id, faction::text AS faction, flags, size, {} FROM gameobject_types",
            (0..24)
                .map(|i| format!("data_{i}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let rows = sqlx::query(&type_sql).fetch_all(pool).await?;
        let mut skipped_types = 0u64;
        for row in rows {
            let label: String = row.get("faction");
            let Some(faction) = CreatureFaction::from_str(&label) else {
                skipped_types += 1;
                continue;
            };
            let entry: i32 = row.get("entry");
            let mut data = [0; 24];
            for (i, slot) in data.iter_mut().enumerate() {
                *slot = row.get(7 + i);
            }
            self.gameobject_types.insert(
                entry as u32,
                GameObjectTypeRow {
                    entry: entry as u32,
                    name: row.get("name"),
                    object_type: row.get("object_type"),
                    display_id: row.get("display_id"),
                    faction: faction.as_protocol() as i32,
                    flags: row.get("flags"),
                    size: row.get("size"),
                    data,
                },
            );
        }
        if skipped_types > 0 {
            tracing::warn!(
                skipped_types,
                "skipped gameobject_types with unknown faction"
            );
        }
        let spawns = sqlx::query(
            "SELECT guid, entry, map_id::text AS map_id, x, y, z, orientation,
                    rotation_0, rotation_1, rotation_2, rotation_3,
                    respawn_secs, anim_progress, state
             FROM gameobject_spawns",
        )
        .fetch_all(pool)
        .await?;
        let mut skipped = 0u64;
        for row in spawns {
            let label: String = row.get("map_id");
            let Some(map) = CharacterMap::from_str(&label) else {
                skipped += 1;
                continue;
            };
            let map_id = map.as_protocol();
            self.gameobject_spawns
                .entry(map_id)
                .or_default()
                .push(GameObjectSpawnRow {
                    guid: row.get::<i32, _>("guid") as u32,
                    entry: row.get::<i32, _>("entry") as u32,
                    map_id,
                    position: Position {
                        x: row.get("x"),
                        y: row.get("y"),
                        z: row.get("z"),
                        orientation: row.get("orientation"),
                    },
                    rotation: [
                        row.get("rotation_0"),
                        row.get("rotation_1"),
                        row.get("rotation_2"),
                        row.get("rotation_3"),
                    ],
                    respawn_secs: row.get("respawn_secs"),
                    anim_progress: row.get("anim_progress"),
                    state: row.get("state"),
                });
        }
        if skipped > 0 {
            tracing::warn!(skipped, "skipped gameobject_spawns with unknown map_id");
        }
        Ok(())
    }
}
