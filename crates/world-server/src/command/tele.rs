use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sqlx::{PgPool, Row};
use wow_shared::{CharacterMap, DbEnum, MAP_EASTERN_KINGDOMS, MAP_KALIMDOR, Position};

#[derive(Debug, Clone)]
pub struct TeleLocation {
    pub name: String,
    pub map_id: u32,
    pub position: Position,
}

#[derive(Clone)]
pub struct TeleStore {
    inner: Arc<Mutex<HashMap<String, TeleLocation>>>,
    pool: Option<PgPool>,
}

impl TeleStore {
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(Mutex::new(builtin())),
            pool: None,
        }
    }

    pub async fn postgres(pool: PgPool) -> anyhow::Result<Self> {
        let store = Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            pool: Some(pool.clone()),
        };
        store.reload().await?;
        if store.list().is_empty() {
            *store.inner.lock().expect("tele mutex") = builtin();
        }
        Ok(store)
    }

    async fn reload(&self) -> anyhow::Result<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };
        let rows =
            sqlx::query("SELECT name, map_id::text AS map_id, x, y, z, orientation FROM game_tele")
                .fetch_all(pool)
                .await?;
        let mut map = HashMap::new();
        for row in rows {
            let name: String = row.get("name");
            let label: String = row.get("map_id");
            let Some(map_id) = CharacterMap::from_str(&label) else {
                continue;
            };
            map.insert(
                name.to_ascii_lowercase(),
                TeleLocation {
                    name,
                    map_id: map_id.as_protocol(),
                    position: Position {
                        x: row.get("x"),
                        y: row.get("y"),
                        z: row.get("z"),
                        orientation: row.get("orientation"),
                    },
                },
            );
        }
        *self.inner.lock().expect("tele mutex") = map;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<TeleLocation> {
        self.inner
            .lock()
            .expect("tele mutex")
            .get(&name.to_ascii_lowercase())
            .cloned()
    }

    pub fn lookup(&self, query: &str) -> Vec<TeleLocation> {
        let query = query.to_ascii_lowercase();
        let mut matches: Vec<_> = self
            .inner
            .lock()
            .expect("tele mutex")
            .values()
            .filter(|loc| loc.name.to_ascii_lowercase().contains(&query))
            .cloned()
            .collect();
        matches.sort_by(|a, b| a.name.cmp(&b.name));
        matches.truncate(20);
        matches
    }

    pub fn list(&self) -> Vec<TeleLocation> {
        let mut list: Vec<_> = self
            .inner
            .lock()
            .expect("tele mutex")
            .values()
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub async fn add(&self, name: &str, map_id: u32, position: Position) -> anyhow::Result<bool> {
        let Some(map) = CharacterMap::from_protocol(map_id) else {
            anyhow::bail!("unknown map id {map_id}");
        };
        let key = name.to_ascii_lowercase();
        {
            let mut inner = self.inner.lock().expect("tele mutex");
            if inner.contains_key(&key) {
                return Ok(false);
            }
            inner.insert(
                key.clone(),
                TeleLocation {
                    name: name.to_string(),
                    map_id,
                    position,
                },
            );
        }
        if let Some(pool) = &self.pool {
            sqlx::query(
                "INSERT INTO game_tele (name, map_id, x, y, z, orientation)
                 VALUES ($1, $2::character_map, $3, $4, $5, $6)",
            )
            .bind(name)
            .bind(map.as_str())
            .bind(position.x)
            .bind(position.y)
            .bind(position.z)
            .bind(position.orientation)
            .execute(pool)
            .await?;
        }
        Ok(true)
    }

    pub async fn delete(&self, name: &str) -> anyhow::Result<bool> {
        let key = name.to_ascii_lowercase();
        let removed = self
            .inner
            .lock()
            .expect("tele mutex")
            .remove(&key)
            .is_some();
        if removed && let Some(pool) = &self.pool {
            sqlx::query("DELETE FROM game_tele WHERE lower(name) = $1")
                .bind(&key)
                .execute(pool)
                .await?;
        }
        Ok(removed)
    }
}

fn builtin() -> HashMap<String, TeleLocation> {
    let spots = [
        (
            "Stormwind",
            MAP_EASTERN_KINGDOMS,
            -8833.38,
            622.738,
            93.7444,
            0.7,
        ),
        (
            "Ironforge",
            MAP_EASTERN_KINGDOMS,
            -4981.25,
            -881.542,
            501.66,
            0.38,
        ),
        ("Darnassus", MAP_KALIMDOR, 9949.56, 2482.58, 1316.18, 4.05),
        ("Orgrimmar", MAP_KALIMDOR, 1633.33, -4439.31, 15.4499, 3.61),
        (
            "ThunderBluff",
            MAP_KALIMDOR,
            -1277.37,
            124.804,
            131.287,
            5.22,
        ),
        (
            "Undercity",
            MAP_EASTERN_KINGDOMS,
            1584.07,
            241.987,
            -52.1534,
            3.14,
        ),
        (
            "Northshire",
            MAP_EASTERN_KINGDOMS,
            -8949.95,
            -132.493,
            83.5312,
            0.0,
        ),
        (
            "Goldshire",
            MAP_EASTERN_KINGDOMS,
            -9465.57,
            72.773,
            56.258,
            4.4,
        ),
        (
            "Coldridge",
            MAP_EASTERN_KINGDOMS,
            -6240.32,
            331.033,
            382.758,
            6.17716,
        ),
        (
            "Deathknell",
            MAP_EASTERN_KINGDOMS,
            1676.71,
            1678.31,
            121.67,
            2.70526,
        ),
        (
            "ValleyOfTrials",
            MAP_KALIMDOR,
            -618.518,
            -4251.67,
            38.718,
            0.0,
        ),
        ("RazorHill", MAP_KALIMDOR, 321.741, -4734.4, 9.49, 4.71),
        ("Crossroads", MAP_KALIMDOR, -456.263, -2652.7, 95.615, 1.57),
        (
            "Shadowglen",
            MAP_KALIMDOR,
            10311.3,
            832.463,
            1326.41,
            5.69632,
        ),
        ("CampNarache", MAP_KALIMDOR, -2917.58, -257.98, 52.9968, 0.0),
        (
            "BootyBay",
            MAP_EASTERN_KINGDOMS,
            -14297.2,
            530.993,
            8.779,
            4.4,
        ),
    ];
    spots
        .into_iter()
        .map(|(name, map_id, x, y, z, orientation)| {
            (
                name.to_ascii_lowercase(),
                TeleLocation {
                    name: name.to_string(),
                    map_id,
                    position: Position {
                        x,
                        y,
                        z,
                        orientation,
                    },
                },
            )
        })
        .collect()
}
