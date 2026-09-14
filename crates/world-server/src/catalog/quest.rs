use sqlx::{PgPool, Row};
use wow_shared::{CharacterClass, CharacterRace, QuestType};

use super::Catalog;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuestRow {
    pub entry: u32,
    pub title: String,
    pub details: String,
    pub objectives: String,
    pub offer_reward_text: String,
    pub request_items_text: String,
    pub end_text: String,
    pub objective_texts: [String; 4],
    pub min_level: i16,
    pub quest_level: i16,
    pub zone_or_sort: i32,
    pub quest_type: QuestType,
    pub required_races: Vec<CharacterRace>,
    pub required_classes: Vec<CharacterClass>,
    pub prev_quest_id: i32,
    pub next_quest_id: i32,
    pub exclusive_group: i32,
    pub next_quest_in_chain: i32,
    pub method: i16,
    pub quest_flags: i32,
    pub special_flags: i32,
    pub src_item_id: i32,
    pub req_item_id: [i32; 4],
    pub req_item_count: [i32; 4],
    pub req_creature_or_go_id: [i32; 4],
    pub req_creature_or_go_count: [i32; 4],
    pub rew_money: i32,
    pub rew_money_max_level: i32,
    pub rew_item_id: [i32; 4],
    pub rew_item_count: [i32; 4],
    pub rew_choice_item_id: [i32; 6],
    pub rew_choice_item_count: [i32; 6],
    pub rew_spell: i32,
    pub rew_spell_cast: i32,
    pub point_map_id: i32,
    pub point_x: f32,
    pub point_y: f32,
}

impl QuestRow {
    pub fn offered_to(&self, race: CharacterRace, class: CharacterClass, level: i32) -> bool {
        if self.min_level > 0 && level < i32::from(self.min_level) {
            return false;
        }
        if !self.required_races.is_empty() && !self.required_races.contains(&race) {
            return false;
        }
        if !self.required_classes.is_empty() && !self.required_classes.contains(&class) {
            return false;
        }
        true
    }

    pub fn kill_objectives(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.req_creature_or_go_id
            .iter()
            .zip(self.req_creature_or_go_count)
            .filter_map(|(id, count)| {
                if *id > 0 && count > 0 {
                    Some((*id as u32, count as u32))
                } else {
                    None
                }
            })
    }

    pub fn auto_complete(&self) -> bool {
        self.method == 0 || self.kill_objectives().next().is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuestgiverKind {
    Creature,
    GameObject,
}

impl Catalog {
    pub fn quest(&self, entry: u32) -> Option<&QuestRow> {
        self.quests.get(&entry)
    }

    pub fn creature_quest_starts(&self, entry: u32) -> &[u32] {
        self.creature_quest_starts
            .get(&entry)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn creature_quest_ends(&self, entry: u32) -> &[u32] {
        self.creature_quest_ends
            .get(&entry)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn questgiver_greeting(&self, kind: QuestgiverKind, entry: u32) -> Option<&str> {
        self.questgiver_greetings
            .get(&(kind, entry))
            .map(String::as_str)
    }

    pub(crate) async fn load_quests(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT entry, title, details, objectives, offer_reward_text, request_items_text,
                    end_text, objective_text_1, objective_text_2, objective_text_3, objective_text_4,
                    min_level, quest_level, zone_or_sort, quest_type::text AS quest_type,
                    required_races::text[] AS required_races, required_classes::text[] AS required_classes,
                    prev_quest_id, next_quest_id, exclusive_group, next_quest_in_chain, method,
                    quest_flags, special_flags, src_item_id,
                    req_item_id_1, req_item_id_2, req_item_id_3, req_item_id_4,
                    req_item_count_1, req_item_count_2, req_item_count_3, req_item_count_4,
                    req_creature_or_go_id_1, req_creature_or_go_id_2, req_creature_or_go_id_3,
                    req_creature_or_go_id_4, req_creature_or_go_count_1, req_creature_or_go_count_2,
                    req_creature_or_go_count_3, req_creature_or_go_count_4,
                    rew_money, rew_money_max_level,
                    rew_item_id_1, rew_item_id_2, rew_item_id_3, rew_item_id_4,
                    rew_item_count_1, rew_item_count_2, rew_item_count_3, rew_item_count_4,
                    rew_choice_item_id_1, rew_choice_item_id_2, rew_choice_item_id_3,
                    rew_choice_item_id_4, rew_choice_item_id_5, rew_choice_item_id_6,
                    rew_choice_item_count_1, rew_choice_item_count_2, rew_choice_item_count_3,
                    rew_choice_item_count_4, rew_choice_item_count_5, rew_choice_item_count_6,
                    rew_spell, rew_spell_cast, point_map_id, point_x, point_y
             FROM quests",
        )
        .fetch_all(pool)
        .await?;
        for row in rows {
            let type_label: String = row.get("quest_type");
            let quest_type = QuestType::from_str(&type_label).unwrap_or(QuestType::None);
            let races: Vec<String> = row.get("required_races");
            let classes: Vec<String> = row.get("required_classes");
            let entry: i32 = row.get("entry");
            self.quests.insert(
                entry as u32,
                QuestRow {
                    entry: entry as u32,
                    title: row.get("title"),
                    details: row.get("details"),
                    objectives: row.get("objectives"),
                    offer_reward_text: row.get("offer_reward_text"),
                    request_items_text: row.get("request_items_text"),
                    end_text: row.get("end_text"),
                    objective_texts: [
                        row.get("objective_text_1"),
                        row.get("objective_text_2"),
                        row.get("objective_text_3"),
                        row.get("objective_text_4"),
                    ],
                    min_level: row.get("min_level"),
                    quest_level: row.get("quest_level"),
                    zone_or_sort: row.get("zone_or_sort"),
                    quest_type,
                    required_races: races
                        .into_iter()
                        .filter_map(|label| CharacterRace::from_str(&label))
                        .collect(),
                    required_classes: classes
                        .into_iter()
                        .filter_map(|label| CharacterClass::from_str(&label))
                        .collect(),
                    prev_quest_id: row.get("prev_quest_id"),
                    next_quest_id: row.get("next_quest_id"),
                    exclusive_group: row.get("exclusive_group"),
                    next_quest_in_chain: row.get("next_quest_in_chain"),
                    method: row.get("method"),
                    quest_flags: row.get("quest_flags"),
                    special_flags: row.get("special_flags"),
                    src_item_id: row.get("src_item_id"),
                    req_item_id: numbered4(&row, "req_item_id"),
                    req_item_count: numbered4(&row, "req_item_count"),
                    req_creature_or_go_id: numbered4(&row, "req_creature_or_go_id"),
                    req_creature_or_go_count: numbered4(&row, "req_creature_or_go_count"),
                    rew_money: row.get("rew_money"),
                    rew_money_max_level: row.get("rew_money_max_level"),
                    rew_item_id: numbered4(&row, "rew_item_id"),
                    rew_item_count: numbered4(&row, "rew_item_count"),
                    rew_choice_item_id: numbered6(&row, "rew_choice_item_id"),
                    rew_choice_item_count: numbered6(&row, "rew_choice_item_count"),
                    rew_spell: row.get("rew_spell"),
                    rew_spell_cast: row.get("rew_spell_cast"),
                    point_map_id: row.get("point_map_id"),
                    point_x: row.get("point_x"),
                    point_y: row.get("point_y"),
                },
            );
        }
        self.load_quest_links(pool, "creature_quest_starts").await?;
        self.load_quest_links(pool, "creature_quest_ends").await?;
        self.load_quest_links(pool, "gameobject_quest_starts")
            .await?;
        self.load_quest_links(pool, "gameobject_quest_ends").await?;
        let areatriggers =
            sqlx::query_as::<_, (i32, i32)>("SELECT entry, quest_id FROM areatrigger_quest_ends")
                .fetch_all(pool)
                .await?;
        for (entry, quest_id) in areatriggers {
            self.areatrigger_quest_ends
                .entry(entry as u32)
                .or_default()
                .push(quest_id as u32);
        }
        let greetings =
            sqlx::query("SELECT entry, kind::text AS kind, text FROM questgiver_greetings")
                .fetch_all(pool)
                .await?;
        for row in greetings {
            let kind = match row.get::<String, _>("kind").as_str() {
                "CREATURE" => QuestgiverKind::Creature,
                "GAMEOBJECT" => QuestgiverKind::GameObject,
                _ => continue,
            };
            let entry: i32 = row.get("entry");
            self.questgiver_greetings
                .insert((kind, entry as u32), row.get("text"));
        }
        Ok(())
    }

    pub(crate) async fn load_quest_links(
        &mut self,
        pool: &PgPool,
        table: &str,
    ) -> anyhow::Result<()> {
        let sql = format!("SELECT entry, quest_id FROM {table}");
        let rows = sqlx::query_as::<_, (i32, i32)>(&sql)
            .fetch_all(pool)
            .await?;
        let dest = match table {
            "creature_quest_starts" => &mut self.creature_quest_starts,
            "creature_quest_ends" => &mut self.creature_quest_ends,
            "gameobject_quest_starts" => &mut self.gameobject_quest_starts,
            "gameobject_quest_ends" => &mut self.gameobject_quest_ends,
            _ => unreachable!(),
        };
        for (entry, quest_id) in rows {
            dest.entry(entry as u32).or_default().push(quest_id as u32);
        }
        Ok(())
    }
}

fn numbered4(row: &sqlx::postgres::PgRow, prefix: &str) -> [i32; 4] {
    let names = [
        format!("{prefix}_1"),
        format!("{prefix}_2"),
        format!("{prefix}_3"),
        format!("{prefix}_4"),
    ];
    std::array::from_fn(|i| row.get(names[i].as_str()))
}

fn numbered6(row: &sqlx::postgres::PgRow, prefix: &str) -> [i32; 6] {
    let names = [
        format!("{prefix}_1"),
        format!("{prefix}_2"),
        format!("{prefix}_3"),
        format!("{prefix}_4"),
        format!("{prefix}_5"),
        format!("{prefix}_6"),
    ];
    std::array::from_fn(|i| row.get(names[i].as_str()))
}

#[cfg(test)]
impl Catalog {
    pub fn insert_quest(&mut self, quest: QuestRow) {
        self.quests.insert(quest.entry, quest);
    }

    pub fn link_quest_start(&mut self, npc_entry: u32, quest_id: u32) {
        self.creature_quest_starts
            .entry(npc_entry)
            .or_default()
            .push(quest_id);
    }

    pub fn link_quest_end(&mut self, npc_entry: u32, quest_id: u32) {
        self.creature_quest_ends
            .entry(npc_entry)
            .or_default()
            .push(quest_id);
    }
}
