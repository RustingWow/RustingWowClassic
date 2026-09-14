use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use sqlx::{PgPool, Row};
use wow_shared::{
    Appearance, CharacterArea, CharacterClass, CharacterGender, CharacterMap, CharacterRace,
    CharacterTemplate, DbEnum, Position,
};

use crate::appearance::{normalize_character_name, race_allowed};
use crate::race_starts::{RaceStart, RaceStarts};

const ACCOUNT_CHARACTER_LIMIT: usize = 10;

const CHARACTER_SELECT: &str = "SELECT id, name, race::text AS race, class::text AS class,
        gender::text AS gender, skin, face, hair_style, hair_color, facial_hair,
        map_id::text AS map_id, x, y, z, orientation, area::text AS area, first_login
     FROM characters";

#[derive(Debug)]
pub enum CreateCharacterError {
    InvalidName,
    NameInUse,
    AccountLimit,
    Disabled,
    Store(anyhow::Error),
}

impl std::fmt::Display for CreateCharacterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName => write!(f, "invalid character name"),
            Self::NameInUse => write!(f, "character name in use"),
            Self::AccountLimit => write!(f, "account character limit reached"),
            Self::Disabled => write!(f, "race or class combination is not allowed"),
            Self::Store(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CreateCharacterError {}

impl From<anyhow::Error> for CreateCharacterError {
    fn from(error: anyhow::Error) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug, Clone)]
pub struct CharacterDraft {
    pub account_id: i64,
    pub name: String,
    pub race: CharacterRace,
    pub class: CharacterClass,
    pub gender: CharacterGender,
    pub appearance: Appearance,
}

#[derive(Clone)]
pub struct CharacterStore {
    inner: CharacterStoreInner,
    starts: Arc<RaceStarts>,
}

#[derive(Clone)]
enum CharacterStoreInner {
    Memory(Arc<Mutex<MemoryCharacters>>),
    Postgres(PgPool),
}

struct MemoryCharacters {
    next_id: AtomicI64,
    by_guid: HashMap<u64, CharacterTemplate>,
    by_name: HashMap<String, u64>,
    by_account: HashMap<i64, Vec<u64>>,
    quests: HashMap<u64, HashMap<u32, StoredQuest>>,
}

#[derive(Clone)]
struct StoredQuest {
    status: StoredQuestStatus,
    kills: [u32; 4],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StoredQuestStatus {
    Active,
    Rewarded,
}

impl CharacterStore {
    pub fn memory() -> Self {
        Self {
            inner: CharacterStoreInner::Memory(Arc::new(Mutex::new(MemoryCharacters {
                next_id: AtomicI64::new(1),
                by_guid: HashMap::new(),
                by_name: HashMap::new(),
                by_account: HashMap::new(),
                quests: HashMap::new(),
            }))),
            starts: Arc::new(RaceStarts::builtin()),
        }
    }

    pub async fn postgres(pool: PgPool) -> anyhow::Result<Self> {
        let starts = RaceStarts::load(&pool).await?;
        Ok(Self {
            inner: CharacterStoreInner::Postgres(pool),
            starts: Arc::new(starts),
        })
    }

    pub async fn list(&self, account_id: i64) -> anyhow::Result<Vec<CharacterTemplate>> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let inner = memory.lock().expect("character store mutex");
                let guids = inner
                    .by_account
                    .get(&account_id)
                    .cloned()
                    .unwrap_or_default();
                Ok(guids
                    .into_iter()
                    .filter_map(|guid| inner.by_guid.get(&guid).cloned())
                    .collect())
            }
            CharacterStoreInner::Postgres(pool) => {
                let rows = sqlx::query(&format!(
                    "{CHARACTER_SELECT} WHERE account_id = $1 ORDER BY id"
                ))
                .bind(account_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(row_to_character).collect()
            }
        }
    }

    pub async fn get(
        &self,
        account_id: i64,
        guid: u64,
    ) -> anyhow::Result<Option<CharacterTemplate>> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let inner = memory.lock().expect("character store mutex");
                let Some(guids) = inner.by_account.get(&account_id) else {
                    return Ok(None);
                };
                if !guids.contains(&guid) {
                    return Ok(None);
                }
                Ok(inner.by_guid.get(&guid).cloned())
            }
            CharacterStoreInner::Postgres(pool) => {
                let row = sqlx::query(&format!(
                    "{CHARACTER_SELECT} WHERE id = $1 AND account_id = $2"
                ))
                .bind(guid as i64)
                .bind(account_id)
                .fetch_optional(pool)
                .await?;
                row.map(row_to_character).transpose()
            }
        }
    }

    pub async fn get_by_guid(&self, guid: u64) -> anyhow::Result<Option<CharacterTemplate>> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let inner = memory.lock().expect("character store mutex");
                Ok(inner.by_guid.get(&guid).cloned())
            }
            CharacterStoreInner::Postgres(pool) => {
                let row = sqlx::query(&format!("{CHARACTER_SELECT} WHERE id = $1"))
                    .bind(guid as i64)
                    .fetch_optional(pool)
                    .await?;
                row.map(row_to_character).transpose()
            }
        }
    }

    pub async fn create(
        &self,
        draft: CharacterDraft,
    ) -> Result<CharacterTemplate, CreateCharacterError> {
        let name =
            normalize_character_name(&draft.name).ok_or(CreateCharacterError::InvalidName)?;
        if !race_allowed(draft.race, draft.class) {
            return Err(CreateCharacterError::Disabled);
        }
        let spawn = self.starts.get(draft.race)?;
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let mut inner = memory.lock().expect("character store mutex");
                insert_memory(&mut inner, draft.account_id, name, &draft, spawn)
            }
            CharacterStoreInner::Postgres(pool) => {
                insert_postgres(pool, draft.account_id, name, &draft, spawn).await
            }
        }
    }

    pub async fn save_position(
        &self,
        guid: u64,
        map_id: CharacterMap,
        position: Position,
    ) -> anyhow::Result<()> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let mut inner = memory.lock().expect("character store mutex");
                if let Some(character) = inner.by_guid.get_mut(&guid) {
                    character.map_id = map_id;
                    character.position = position;
                }
                Ok(())
            }
            CharacterStoreInner::Postgres(pool) => {
                sqlx::query(
                    "UPDATE characters
                     SET map_id = $2::character_map, x = $3, y = $4, z = $5, orientation = $6, updated_at = NOW()
                     WHERE id = $1",
                )
                .bind(guid as i64)
                .bind(map_id.as_str())
                .bind(position.x)
                .bind(position.y)
                .bind(position.z)
                .bind(position.orientation)
                .execute(pool)
                .await?;
                Ok(())
            }
        }
    }

    pub async fn mark_entered_world(&self, guid: u64) -> anyhow::Result<()> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let mut inner = memory.lock().expect("character store mutex");
                if let Some(character) = inner.by_guid.get_mut(&guid) {
                    character.first_login = false;
                }
                Ok(())
            }
            CharacterStoreInner::Postgres(pool) => {
                sqlx::query(
                    "UPDATE characters SET first_login = FALSE, updated_at = NOW() WHERE id = $1",
                )
                .bind(guid as i64)
                .execute(pool)
                .await?;
                Ok(())
            }
        }
    }

    pub async fn load_quests(
        &self,
        guid: u64,
    ) -> anyhow::Result<(Vec<crate::player::QuestLogEntry>, Vec<u32>)> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let inner = memory.lock().expect("character store mutex");
                let Some(rows) = inner.quests.get(&guid) else {
                    return Ok((Vec::new(), Vec::new()));
                };
                let mut log = Vec::new();
                let mut rewarded = Vec::new();
                for (quest_id, row) in rows {
                    match row.status {
                        StoredQuestStatus::Active => log.push(crate::player::QuestLogEntry {
                            quest_id: *quest_id,
                            kills: row.kills,
                            complete: false,
                        }),
                        StoredQuestStatus::Rewarded => rewarded.push(*quest_id),
                    }
                }
                Ok((log, rewarded))
            }
            CharacterStoreInner::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT quest_id, status::text AS status, kill_count_0, kill_count_1,
                            kill_count_2, kill_count_3
                     FROM character_quests WHERE character_id = $1",
                )
                .bind(guid as i64)
                .fetch_all(pool)
                .await?;
                let mut log = Vec::new();
                let mut rewarded = Vec::new();
                for row in rows {
                    let quest_id = row.try_get::<i32, _>("quest_id")? as u32;
                    let status: String = row.try_get("status")?;
                    let kills = [
                        row.try_get::<i32, _>("kill_count_0")? as u32,
                        row.try_get::<i32, _>("kill_count_1")? as u32,
                        row.try_get::<i32, _>("kill_count_2")? as u32,
                        row.try_get::<i32, _>("kill_count_3")? as u32,
                    ];
                    if status == "REWARDED" {
                        rewarded.push(quest_id);
                    } else {
                        log.push(crate::player::QuestLogEntry {
                            quest_id,
                            kills,
                            complete: false,
                        });
                    }
                }
                Ok((log, rewarded))
            }
        }
    }

    pub async fn apply_quest_change(
        &self,
        guid: u64,
        change: crate::quest::QuestStateChange,
    ) -> anyhow::Result<()> {
        match change {
            crate::quest::QuestStateChange::Active {
                quest_id,
                kills,
                complete: _,
            } => {
                self.upsert_quest(guid, quest_id, StoredQuestStatus::Active, kills)
                    .await
            }
            crate::quest::QuestStateChange::Rewarded { quest_id } => {
                self.upsert_quest(guid, quest_id, StoredQuestStatus::Rewarded, [0; 4])
                    .await
            }
            crate::quest::QuestStateChange::Removed { quest_id } => {
                self.remove_quest(guid, quest_id).await
            }
        }
    }

    async fn upsert_quest(
        &self,
        guid: u64,
        quest_id: u32,
        status: StoredQuestStatus,
        kills: [u32; 4],
    ) -> anyhow::Result<()> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let mut inner = memory.lock().expect("character store mutex");
                inner
                    .quests
                    .entry(guid)
                    .or_default()
                    .insert(quest_id, StoredQuest { status, kills });
                Ok(())
            }
            CharacterStoreInner::Postgres(pool) => {
                let status_label = match status {
                    StoredQuestStatus::Active => "ACTIVE",
                    StoredQuestStatus::Rewarded => "REWARDED",
                };
                sqlx::query(
                    "INSERT INTO character_quests
                        (character_id, quest_id, status, kill_count_0, kill_count_1, kill_count_2, kill_count_3)
                     VALUES ($1, $2, $3::character_quest_status, $4, $5, $6, $7)
                     ON CONFLICT (character_id, quest_id) DO UPDATE SET
                        status = EXCLUDED.status,
                        kill_count_0 = EXCLUDED.kill_count_0,
                        kill_count_1 = EXCLUDED.kill_count_1,
                        kill_count_2 = EXCLUDED.kill_count_2,
                        kill_count_3 = EXCLUDED.kill_count_3",
                )
                .bind(guid as i64)
                .bind(quest_id as i32)
                .bind(status_label)
                .bind(kills[0] as i32)
                .bind(kills[1] as i32)
                .bind(kills[2] as i32)
                .bind(kills[3] as i32)
                .execute(pool)
                .await?;
                Ok(())
            }
        }
    }

    async fn remove_quest(&self, guid: u64, quest_id: u32) -> anyhow::Result<()> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let mut inner = memory.lock().expect("character store mutex");
                if let Some(rows) = inner.quests.get_mut(&guid) {
                    rows.remove(&quest_id);
                }
                Ok(())
            }
            CharacterStoreInner::Postgres(pool) => {
                sqlx::query(
                    "DELETE FROM character_quests WHERE character_id = $1 AND quest_id = $2",
                )
                .bind(guid as i64)
                .bind(quest_id as i32)
                .execute(pool)
                .await?;
                Ok(())
            }
        }
    }

    pub async fn delete(&self, account_id: i64, guid: u64) -> anyhow::Result<bool> {
        match &self.inner {
            CharacterStoreInner::Memory(memory) => {
                let mut inner = memory.lock().expect("character store mutex");
                let Some(character) = inner.by_guid.get(&guid) else {
                    return Ok(false);
                };
                if character_account_id(&inner, guid) != Some(account_id) {
                    return Ok(false);
                }
                let name_key = character.name.to_ascii_lowercase();
                inner.by_guid.remove(&guid);
                inner.by_name.remove(&name_key);
                inner.quests.remove(&guid);
                if let Some(guids) = inner.by_account.get_mut(&account_id) {
                    guids.retain(|id| *id != guid);
                }
                Ok(true)
            }
            CharacterStoreInner::Postgres(pool) => {
                let result =
                    sqlx::query("DELETE FROM characters WHERE id = $1 AND account_id = $2")
                        .bind(guid as i64)
                        .bind(account_id)
                        .execute(pool)
                        .await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }
}

fn character_account_id(inner: &MemoryCharacters, guid: u64) -> Option<i64> {
    inner
        .by_account
        .iter()
        .find_map(|(account_id, guids)| guids.contains(&guid).then_some(*account_id))
}

fn insert_memory(
    inner: &mut MemoryCharacters,
    account_id: i64,
    name: String,
    draft: &CharacterDraft,
    spawn: RaceStart,
) -> Result<CharacterTemplate, CreateCharacterError> {
    let name_key = name.to_ascii_lowercase();
    if inner.by_name.contains_key(&name_key) {
        return Err(CreateCharacterError::NameInUse);
    }
    if inner
        .by_account
        .get(&account_id)
        .is_some_and(|chars| chars.len() >= ACCOUNT_CHARACTER_LIMIT)
    {
        return Err(CreateCharacterError::AccountLimit);
    }
    let guid = inner.next_id.fetch_add(1, Ordering::Relaxed) as u64;
    let character = CharacterTemplate {
        guid,
        name,
        race: draft.race,
        class: draft.class,
        gender: draft.gender,
        appearance: draft.appearance,
        map_id: spawn.map_id,
        position: spawn.position,
        area: spawn.area,
        first_login: true,
    };
    inner.by_name.insert(name_key, guid);
    inner.by_guid.insert(guid, character.clone());
    inner.by_account.entry(account_id).or_default().push(guid);
    Ok(character)
}

async fn insert_postgres(
    pool: &PgPool,
    account_id: i64,
    name: String,
    draft: &CharacterDraft,
    spawn: RaceStart,
) -> Result<CharacterTemplate, CreateCharacterError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(pool)
        .await
        .map_err(|error| CreateCharacterError::Store(error.into()))?;
    if count as usize >= ACCOUNT_CHARACTER_LIMIT {
        return Err(CreateCharacterError::AccountLimit);
    }
    let result = sqlx::query(
        "INSERT INTO characters
            (account_id, name, race, class, gender, skin, face, hair_style, hair_color, facial_hair,
             map_id, x, y, z, orientation, area)
         VALUES (
            $1, $2, $3::character_race, $4::character_class, $5::character_gender,
            $6, $7, $8, $9, $10, $11::character_map, $12, $13, $14, $15, $16::character_area
         )
         RETURNING id, name, race::text AS race, class::text AS class, gender::text AS gender,
                   skin, face, hair_style, hair_color, facial_hair, map_id::text AS map_id,
                   x, y, z, orientation, area::text AS area, first_login",
    )
    .bind(account_id)
    .bind(&name)
    .bind(draft.race.as_str())
    .bind(draft.class.as_str())
    .bind(draft.gender.as_str())
    .bind(draft.appearance.skin as i16)
    .bind(draft.appearance.face as i16)
    .bind(draft.appearance.hair_style as i16)
    .bind(draft.appearance.hair_color as i16)
    .bind(draft.appearance.facial_hair as i16)
    .bind(spawn.map_id.as_str())
    .bind(spawn.position.x)
    .bind(spawn.position.y)
    .bind(spawn.position.z)
    .bind(spawn.position.orientation)
    .bind(spawn.area.as_str())
    .fetch_one(pool)
    .await;
    match result {
        Ok(row) => row_to_character(row).map_err(CreateCharacterError::Store),
        Err(error) if is_unique_violation(&error) => Err(CreateCharacterError::NameInUse),
        Err(error) => Err(CreateCharacterError::Store(error.into())),
    }
}

fn row_to_character(row: sqlx::postgres::PgRow) -> anyhow::Result<CharacterTemplate> {
    Ok(CharacterTemplate {
        guid: row.try_get::<i64, _>("id")? as u64,
        name: row.try_get("name")?,
        race: parse_db_enum(row.try_get("race")?, CharacterRace::from_str, "race")?,
        class: parse_db_enum(row.try_get("class")?, CharacterClass::from_str, "class")?,
        gender: parse_db_enum(row.try_get("gender")?, CharacterGender::from_str, "gender")?,
        appearance: Appearance {
            skin: row.try_get::<i16, _>("skin")? as u8,
            face: row.try_get::<i16, _>("face")? as u8,
            hair_style: row.try_get::<i16, _>("hair_style")? as u8,
            hair_color: row.try_get::<i16, _>("hair_color")? as u8,
            facial_hair: row.try_get::<i16, _>("facial_hair")? as u8,
        },
        map_id: parse_db_enum(row.try_get("map_id")?, CharacterMap::from_str, "map")?,
        position: Position {
            x: row.try_get("x")?,
            y: row.try_get("y")?,
            z: row.try_get("z")?,
            orientation: row.try_get("orientation")?,
        },
        area: parse_db_enum(row.try_get("area")?, CharacterArea::from_str, "area")?,
        first_login: row.try_get("first_login")?,
    })
}

fn parse_db_enum<T>(value: String, parse: fn(&str) -> Option<T>, what: &str) -> anyhow::Result<T> {
    parse(&value).ok_or_else(|| anyhow::anyhow!("unknown {what} in database: {value}"))
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| code == "23505")
}

#[cfg(test)]
#[path = "../test/character_store.rs"]
mod tests;
