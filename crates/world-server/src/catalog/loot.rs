use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use super::Catalog;

#[derive(Clone, Debug)]
pub(crate) struct LootRow {
    pub item_or_ref: i32,
    pub chance: f32,
    pub group_id: i16,
    pub min_count: i32,
    pub max_count: i32,
    #[allow(dead_code)]
    pub condition_id: i32,
}

impl Catalog {
    pub fn roll_loot(&self, loot_id: i32) -> Vec<(u32, u32)> {
        let mut drops = Vec::new();
        roll_table(&self.loot, &self.loot_refs, loot_id, &mut drops, 0);
        drops
    }

    pub(crate) fn loot_row_count(tables: &HashMap<i32, Vec<LootRow>>) -> usize {
        tables.values().map(Vec::len).sum()
    }

    pub(crate) async fn load_loot(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM creature_loot",
            &mut self.loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT ref_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM loot_references",
            &mut self.loot_refs,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM gameobject_loot",
            &mut self.gameobject_loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM item_loot",
            &mut self.item_loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM fishing_loot",
            &mut self.fishing_loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM skinning_loot",
            &mut self.skinning_loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM pickpocket_loot",
            &mut self.pickpocket_loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM disenchant_loot",
            &mut self.disenchant_loot,
        )
        .await?;
        load_loot_map(
            pool,
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count, condition_id FROM mail_loot",
            &mut self.mail_loot,
        )
        .await?;
        Ok(())
    }
}

async fn load_loot_map(
    pool: &PgPool,
    sql: &str,
    dest: &mut HashMap<i32, Vec<LootRow>>,
) -> anyhow::Result<()> {
    let rows = sqlx::query_as::<_, (i32, i32, f32, i16, i32, i32, i32)>(sql)
        .fetch_all(pool)
        .await?;
    for row in rows {
        dest.entry(row.0).or_default().push(LootRow {
            item_or_ref: row.1,
            chance: row.2,
            group_id: row.3,
            min_count: row.4,
            max_count: row.5,
            condition_id: row.6,
        });
    }
    Ok(())
}

fn roll_table(
    tables: &HashMap<i32, Vec<LootRow>>,
    refs: &HashMap<i32, Vec<LootRow>>,
    id: i32,
    out: &mut Vec<(u32, u32)>,
    depth: u8,
) {
    if id == 0 || depth > 8 {
        return;
    }
    let Some(rows) = tables.get(&id).or_else(|| refs.get(&id)) else {
        return;
    };
    let mut grouped: HashMap<i16, Vec<&LootRow>> = HashMap::new();
    for row in rows {
        grouped.entry(row.group_id).or_default().push(row);
    }
    for (group, group_rows) in grouped {
        if group == 0 {
            for row in group_rows {
                consider_row(tables, refs, row, out, depth);
            }
        } else if let Some(row) = pick_group(&group_rows) {
            consider_row(tables, refs, row, out, depth);
        }
    }
}

fn consider_row(
    _tables: &HashMap<i32, Vec<LootRow>>,
    refs: &HashMap<i32, Vec<LootRow>>,
    row: &LootRow,
    out: &mut Vec<(u32, u32)>,
    depth: u8,
) {
    let chance = row.chance.abs();
    if chance <= 0.0 || !roll_pct(chance) {
        return;
    }
    if row.item_or_ref < 0 {
        roll_table(refs, refs, row.item_or_ref.abs(), out, depth + 1);
        return;
    }
    if row.item_or_ref == 0 {
        return;
    }
    let span = (row.max_count - row.min_count).max(0) as u32;
    let extra = if span == 0 {
        0
    } else {
        cheap_rand() % (span + 1)
    };
    let count = (row.min_count.max(1) as u32) + extra;
    out.push((row.item_or_ref as u32, count));
}

fn pick_group<'a>(rows: &[&'a LootRow]) -> Option<&'a LootRow> {
    let total: f32 = rows.iter().map(|row| row.chance.abs().max(0.0)).sum();
    if total <= 0.0 {
        return rows.first().copied();
    }
    let mut cursor = (cheap_rand() as f32 / u32::MAX as f32) * total;
    for row in rows {
        cursor -= row.chance.abs().max(0.0);
        if cursor <= 0.0 {
            return Some(*row);
        }
    }
    rows.last().copied()
}

fn roll_pct(chance: f32) -> bool {
    if chance >= 100.0 {
        return true;
    }
    (cheap_rand() % 10000) as f32 / 100.0 < chance
}

fn cheap_rand() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(1);
    nanos.wrapping_mul(1664525).wrapping_add(1013904223)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loot_row(item_or_ref: i32, chance: f32, min_count: i32, max_count: i32) -> LootRow {
        LootRow {
            item_or_ref,
            chance,
            group_id: 0,
            min_count,
            max_count,
            condition_id: 0,
        }
    }

    fn grouped(item_or_ref: i32, group_id: i16) -> LootRow {
        LootRow {
            item_or_ref,
            chance: 100.0,
            group_id,
            min_count: 1,
            max_count: 1,
            condition_id: 0,
        }
    }

    #[test]
    fn roll_follows_negative_reference() {
        let mut catalog = Catalog::empty();
        catalog.loot.insert(
            1,
            vec![loot_row(-50, 100.0, 1, 1), loot_row(117, 100.0, 1, 1)],
        );
        catalog
            .loot_refs
            .insert(50, vec![loot_row(414, 100.0, 2, 2)]);
        let drops = catalog.roll_loot(1);
        assert!(drops.contains(&(414, 2)));
        assert!(drops.contains(&(117, 1)));
    }

    #[test]
    fn roll_keeps_the_same_item_in_two_groups() {
        let mut catalog = Catalog::empty();
        catalog
            .loot
            .insert(1, vec![grouped(117, 1), grouped(117, 2)]);
        let drops = catalog.roll_loot(1);
        assert_eq!(drops.iter().filter(|drop| drop.0 == 117).count(), 2);
    }
}
