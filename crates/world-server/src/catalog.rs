use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{PgPool, Row};
use wow_shared::{CharacterMap, CreatureFaction, DbEnum, GossipOptionKind, Position};
use wow_world_messages::vanilla::CreatureFamily;

use crate::creature::{
    CREATURE_TYPE_BEAST, Creature, Gossip, GossipAction, GossipMenu, GossipOption, unit_guid,
};

const SERVICE_NPC_FLAGS: i32 = 0x0000_FFFF;

#[derive(Clone, Debug)]
pub struct CreatureTypeRow {
    pub entry: u32,
    pub name: String,
    pub sub_name: String,
    pub level: i32,
    pub display_id: i32,
    pub faction: i32,
    pub family: u8,
    pub creature_type: u32,
    pub npc_flags: i32,
    pub civilian: bool,
    pub health: i32,
    pub melee_damage: i32,
    pub loot_id: i32,
    pub gossip_menu_id: u32,
    pub vendor_template_id: u32,
}

#[derive(Clone, Debug)]
pub struct SpawnRow {
    pub guid: u32,
    pub entry: u32,
    pub map_id: u32,
    pub position: Position,
    pub respawn_secs: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ItemRow {
    pub entry: u32,
    pub class: u8,
    pub subclass: u8,
    pub name: String,
    pub display_id: u32,
    pub quality: u8,
    pub flags: u32,
    pub buy_count: u32,
    pub buy_price: u32,
    pub sell_price: u32,
    pub inventory_type: u8,
    pub item_level: u8,
    pub required_level: u8,
    pub stackable: u32,
    pub max_durability: u32,
    pub description: String,
    pub delay: u32,
    pub armor: i32,
}

#[derive(Clone, Debug)]
pub struct VendorListing {
    pub item_id: u32,
    pub slot: i16,
    pub maxcount: i16,
}

#[derive(Clone, Debug)]
struct LootRow {
    item_or_ref: i32,
    chance: f32,
    group_id: i16,
    min_count: i32,
    max_count: i32,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    types: HashMap<u32, CreatureTypeRow>,
    spawns: HashMap<u32, Vec<SpawnRow>>,
    items: HashMap<u32, ItemRow>,
    vendors: HashMap<u32, Vec<VendorListing>>,
    vendor_templates: HashMap<u32, Vec<VendorListing>>,
    loot: HashMap<i32, Vec<LootRow>>,
    loot_refs: HashMap<i32, Vec<LootRow>>,
    gossip_menu_text: HashMap<u32, Vec<u32>>,
    gossip_options: HashMap<u32, Vec<(u32, GossipOption, GossipOptionKind)>>,
    gossip_texts: HashMap<u32, String>,
}

impl Catalog {
    pub fn empty() -> Self {
        Self {
            types: HashMap::new(),
            spawns: HashMap::new(),
            items: HashMap::new(),
            vendors: HashMap::new(),
            vendor_templates: HashMap::new(),
            loot: HashMap::new(),
            loot_refs: HashMap::new(),
            gossip_menu_text: HashMap::new(),
            gossip_options: HashMap::new(),
            gossip_texts: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty() || self.spawns.is_empty()
    }

    pub async fn load(pool: &PgPool) -> anyhow::Result<Arc<Self>> {
        let mut catalog = Self::empty();
        catalog.load_types(pool).await?;
        catalog.load_spawns(pool).await?;
        catalog.load_items(pool).await?;
        catalog.load_vendors(pool).await?;
        catalog.load_loot(pool).await?;
        catalog.load_gossip(pool).await?;
        tracing::info!(
            types = catalog.types.len(),
            spawns = catalog.spawns.values().map(Vec::len).sum::<usize>(),
            items = catalog.items.len(),
            "loaded world catalog"
        );
        Ok(Arc::new(catalog))
    }

    pub fn creatures_for_map(&self, map_id: u32) -> Vec<Creature> {
        let Some(spawns) = self.spawns.get(&map_id) else {
            return Vec::new();
        };
        spawns
            .iter()
            .filter_map(|spawn| self.creature_from_spawn(spawn))
            .collect()
    }

    pub fn item(&self, entry: u32) -> Option<&ItemRow> {
        self.items.get(&entry)
    }

    pub fn gossip_text(&self, text_id: u32) -> Option<String> {
        self.gossip_texts.get(&text_id).cloned()
    }

    pub fn vendor_listings(&self, creature_entry: u32) -> Vec<VendorListing> {
        let mut listings = self.vendors.get(&creature_entry).cloned().unwrap_or_default();
        if let Some(kind) = self.types.get(&creature_entry)
            && kind.vendor_template_id != 0
            && let Some(template) = self.vendor_templates.get(&kind.vendor_template_id)
        {
            listings.extend(template.iter().cloned());
        }
        listings.sort_by_key(|listing| listing.slot);
        listings
    }

    pub fn roll_loot(&self, loot_id: i32) -> Vec<(u32, u32)> {
        let mut drops = Vec::new();
        roll_table(&self.loot, &self.loot_refs, loot_id, &mut drops, 0);
        drops
    }

    fn creature_from_spawn(&self, spawn: &SpawnRow) -> Option<Creature> {
        let kind = self.types.get(&spawn.entry)?;
        let gossip = self.gossip(kind.gossip_menu_id);
        let service = kind.npc_flags & SERVICE_NPC_FLAGS != 0;
        let hostile = !kind.civilian && !service;
        Some(Creature {
            guid: unit_guid(spawn.guid, spawn.entry),
            entry: spawn.entry,
            name: kind.name.clone(),
            sub_name: kind.sub_name.clone(),
            display_id: kind.display_id,
            faction: kind.faction,
            position: spawn.position,
            health: kind.health.max(1),
            max_health: kind.health.max(1),
            level: kind.level.max(1),
            npc_flags: kind.npc_flags,
            creature_type: if kind.creature_type == 0 {
                CREATURE_TYPE_BEAST
            } else {
                kind.creature_type
            },
            family: family_from_u32(kind.family as u32),
            civilian: kind.civilian,
            hostile,
            dead: false,
            gossip,
            loot_id: kind.loot_id,
            respawn_secs: spawn.respawn_secs.max(5),
            melee_damage: kind.melee_damage,
        })
    }

    fn gossip(&self, menu_id: u32) -> Option<Gossip> {
        if menu_id == 0 {
            return None;
        }
        let mut menus = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![menu_id];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(text_ids) = self.gossip_menu_text.get(&id) else {
                continue;
            };
            let text_id = *text_ids.first()?;
            let text = self
                .gossip_texts
                .get(&text_id)
                .cloned()
                .unwrap_or_default();
            let mut options = Vec::new();
            if let Some(raw) = self.gossip_options.get(&id) {
                for (option_id, option, kind) in raw {
                    let mut option = option.clone();
                    option.id = *option_id;
                    if *kind == GossipOptionKind::Vendor {
                        option.action = GossipAction::OpenVendor;
                    } else if let GossipAction::ShowMenu { text_id } = option.action {
                        if let Some((next_menu, _)) = self
                            .gossip_menu_text
                            .iter()
                            .find(|(_, texts)| texts.contains(&text_id))
                        {
                            stack.push(*next_menu);
                        }
                    }
                    options.push(option);
                }
            }
            menus.push(GossipMenu {
                text_id,
                text,
                options,
            });
        }
        if menus.is_empty() {
            None
        } else {
            Some(Gossip { menus })
        }
    }

    async fn load_types(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT entry, name, sub_name, min_level, display_id, faction::text AS faction, family,
                    creature_type, npc_flags, civilian, health, melee_damage,
                    loot_id, gossip_menu_id, vendor_template_id FROM creature_types",
        )
        .fetch_all(pool)
        .await?;
        let mut skipped = 0u64;
        for row in rows {
            let label: String = row.get("faction");
            let Some(faction) = CreatureFaction::from_str(&label) else {
                skipped += 1;
                continue;
            };
            let entry: i32 = row.get(0);
            self.types.insert(
                entry as u32,
                CreatureTypeRow {
                    entry: entry as u32,
                    name: row.get(1),
                    sub_name: row.get(2),
                    level: i32::from(row.get::<i16, _>(3)),
                    display_id: row.get(4),
                    faction: faction.as_protocol() as i32,
                    family: row.get::<i16, _>(6) as u8,
                    creature_type: row.get::<i32, _>(7) as u32,
                    npc_flags: row.get(8),
                    civilian: row.get(9),
                    health: row.get(10),
                    melee_damage: row.get(11),
                    loot_id: row.get(12),
                    gossip_menu_id: row.get::<i32, _>(13) as u32,
                    vendor_template_id: row.get::<i32, _>(14) as u32,
                },
            );
        }
        if skipped > 0 {
            tracing::warn!(skipped, "skipped creature_types with unknown faction");
        }
        Ok(())
    }

    async fn load_spawns(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT guid, entry, map_id::text AS map_id, x, y, z, orientation, respawn_secs
             FROM creature_spawns",
        )
        .fetch_all(pool)
        .await?;
        let mut skipped = 0u64;
        for row in rows {
            let label: String = row.get("map_id");
            let Some(map) = CharacterMap::from_str(&label) else {
                skipped += 1;
                continue;
            };
            let map_id = map.as_protocol();
            self.spawns.entry(map_id).or_default().push(SpawnRow {
                guid: row.get::<i32, _>("guid") as u32,
                entry: row.get::<i32, _>("entry") as u32,
                map_id,
                position: Position {
                    x: row.get("x"),
                    y: row.get("y"),
                    z: row.get("z"),
                    orientation: row.get("orientation"),
                },
                respawn_secs: row.get::<i32, _>("respawn_secs") as u32,
            });
        }
        if skipped > 0 {
            tracing::warn!(skipped, "skipped creature_spawns with unknown map_id");
        }
        Ok(())
    }

    async fn load_items(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT entry, class, subclass, name, display_id, quality, flags, buy_count, buy_price,
                    sell_price, inventory_type, item_level, required_level, stackable,
                    max_durability, description, delay, armor FROM items",
        )
        .fetch_all(pool)
        .await?;
        for row in rows {
            let entry: i32 = row.get(0);
            self.items.insert(
                entry as u32,
                ItemRow {
                    entry: entry as u32,
                    class: row.get::<i16, _>(1) as u8,
                    subclass: row.get::<i16, _>(2) as u8,
                    name: row.get(3),
                    display_id: row.get::<i32, _>(4) as u32,
                    quality: row.get::<i16, _>(5) as u8,
                    flags: row.get::<i32, _>(6) as u32,
                    buy_count: row.get::<i16, _>(7) as u32,
                    buy_price: row.get::<i32, _>(8) as u32,
                    sell_price: row.get::<i32, _>(9) as u32,
                    inventory_type: row.get::<i16, _>(10) as u8,
                    item_level: row.get::<i16, _>(11) as u8,
                    required_level: row.get::<i16, _>(12) as u8,
                    stackable: row.get::<i32, _>(13) as u32,
                    max_durability: row.get::<i32, _>(14) as u32,
                    description: row.get(15),
                    delay: row.get::<i32, _>(16) as u32,
                    armor: row.get(17),
                },
            );
        }
        Ok(())
    }

    async fn load_vendors(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let vendors = sqlx::query_as::<_, (i32, i32, i16, i16)>(
            "SELECT creature_entry, item_id, slot, maxcount FROM creature_vendors",
        )
        .fetch_all(pool)
        .await?;
        for row in vendors {
            self.vendors.entry(row.0 as u32).or_default().push(VendorListing {
                item_id: row.1 as u32,
                slot: row.2,
                maxcount: row.3,
            });
        }
        let templates = sqlx::query_as::<_, (i32, i32, i16, i16)>(
            "SELECT template_id, item_id, slot, maxcount FROM vendor_templates",
        )
        .fetch_all(pool)
        .await?;
        for row in templates {
            self.vendor_templates
                .entry(row.0 as u32)
                .or_default()
                .push(VendorListing {
                    item_id: row.1 as u32,
                    slot: row.2,
                    maxcount: row.3,
                });
        }
        Ok(())
    }

    async fn load_loot(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let loot = sqlx::query_as::<_, (i32, i32, f32, i16, i32, i32)>(
            "SELECT loot_id, item_or_ref, chance, group_id, min_count, max_count FROM creature_loot",
        )
        .fetch_all(pool)
        .await?;
        for row in loot {
            self.loot.entry(row.0).or_default().push(LootRow {
                item_or_ref: row.1,
                chance: row.2,
                group_id: row.3,
                min_count: row.4,
                max_count: row.5,
            });
        }
        let refs = sqlx::query_as::<_, (i32, i32, f32, i16, i32, i32)>(
            "SELECT ref_id, item_or_ref, chance, group_id, min_count, max_count FROM loot_references",
        )
        .fetch_all(pool)
        .await?;
        for row in refs {
            self.loot_refs.entry(row.0).or_default().push(LootRow {
                item_or_ref: row.1,
                chance: row.2,
                group_id: row.3,
                min_count: row.4,
                max_count: row.5,
            });
        }
        Ok(())
    }

    async fn load_gossip(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let menus = sqlx::query_as::<_, (i32, i32)>("SELECT menu_id, text_id FROM gossip_menus")
            .fetch_all(pool)
            .await?;
        for row in menus {
            self.gossip_menu_text
                .entry(row.0 as u32)
                .or_default()
                .push(row.1 as u32);
        }
        let texts = sqlx::query_as::<_, (i32, String)>("SELECT text_id, text FROM gossip_texts")
            .fetch_all(pool)
            .await?;
        for row in texts {
            self.gossip_texts.insert(row.0 as u32, row.1);
        }
        let options = sqlx::query(
            "SELECT menu_id, option_id, icon, text, option_kind::text AS option_kind, action_menu_id
             FROM gossip_options",
        )
        .fetch_all(pool)
        .await?;
        let mut skipped = 0u64;
        for row in options {
            let label: String = row.get("option_kind");
            let Some(kind) = GossipOptionKind::from_str(&label) else {
                skipped += 1;
                continue;
            };
            let menu_id: i32 = row.get("menu_id");
            let option_id: i32 = row.get("option_id");
            let action_menu_id: i32 = row.get("action_menu_id");
            let action = if action_menu_id > 0 {
                let next_text = self
                    .gossip_menu_text
                    .get(&(action_menu_id as u32))
                    .and_then(|ids| ids.first().copied())
                    .unwrap_or(action_menu_id as u32);
                GossipAction::ShowMenu { text_id: next_text }
            } else {
                GossipAction::Close
            };
            self.gossip_options
                .entry(menu_id as u32)
                .or_default()
                .push((
                    option_id as u32,
                    GossipOption {
                        id: option_id as u32,
                        text: row.get("text"),
                        action,
                    },
                    kind,
                ));
        }
        if skipped > 0 {
            tracing::warn!(skipped, "skipped gossip_options with unknown option_kind");
        }
        Ok(())
    }
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
    let extra = if span == 0 { 0 } else { cheap_rand() % (span + 1) };
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

fn family_from_u32(value: u32) -> CreatureFamily {
    CreatureFamily::try_from(value as u8).unwrap_or(CreatureFamily::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::NPC_FLAG_VENDOR;

    fn loot_row(item_or_ref: i32, chance: f32, min_count: i32, max_count: i32) -> LootRow {
        LootRow {
            item_or_ref,
            chance,
            group_id: 0,
            min_count,
            max_count,
        }
    }

    #[test]
    fn roll_follows_negative_reference() {
        let mut catalog = Catalog::empty();
        catalog.loot.insert(
            1,
            vec![
                loot_row(-50, 100.0, 1, 1),
                loot_row(117, 100.0, 1, 1),
            ],
        );
        catalog
            .loot_refs
            .insert(50, vec![loot_row(414, 100.0, 2, 2)]);
        let drops = catalog.roll_loot(1);
        assert!(drops.contains(&(414, 2)));
        assert!(drops.contains(&(117, 1)));
    }

    #[test]
    fn vendor_listings_append_the_shared_template() {
        let mut catalog = Catalog::empty();
        catalog.types.insert(
            6,
            CreatureTypeRow {
                entry: 6,
                name: "Kobold".into(),
                sub_name: String::new(),
                level: 1,
                display_id: 1,
                faction: 1,
                family: 0,
                creature_type: 7,
                npc_flags: NPC_FLAG_VENDOR,
                civilian: true,
                health: 30,
                melee_damage: 0,
                loot_id: 0,
                gossip_menu_id: 0,
                vendor_template_id: 9,
            },
        );
        catalog.vendors.insert(
            6,
            vec![VendorListing {
                item_id: 117,
                slot: 0,
                maxcount: 0,
            }],
        );
        catalog.vendor_templates.insert(
            9,
            vec![VendorListing {
                item_id: 159,
                slot: 1,
                maxcount: 0,
            }],
        );
        let listings = catalog.vendor_listings(6);
        assert_eq!(listings.len(), 2);
        assert_eq!(listings[0].item_id, 117);
        assert_eq!(listings[1].item_id, 159);
    }
}
