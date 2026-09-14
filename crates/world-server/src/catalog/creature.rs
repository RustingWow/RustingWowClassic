use sqlx::{PgPool, Row};
use wow_shared::{CharacterMap, CreatureFaction, DbEnum, Position};
use wow_world_messages::vanilla::CreatureFamily;

use crate::creature::{CREATURE_TYPE_BEAST, CREATURE_TYPE_CRITTER, Creature, unit_guid};

use super::Catalog;

pub(crate) const SERVICE_NPC_FLAGS: i32 = 0x0000_FFFF;
/// Vanilla `UNIT_FLAG_*` bits that mean this creature must not aggro players.
const UNIT_FLAGS_NO_PLAYER_AGGRO: i32 = 0x0000_0002 // NON_ATTACKABLE
    | 0x0000_0080 // NOT_ATTACKABLE_1
    | 0x0000_0100 // OOC_NOT_ATTACKABLE
    | 0x0000_0200; // PASSIVE

#[derive(Clone, Debug)]
pub struct CreatureTypeRow {
    pub entry: u32,
    pub name: String,
    pub sub_name: String,
    pub level: i32,
    pub max_level: i32,
    pub display_id: i32,
    pub faction: i32,
    pub family: u8,
    pub creature_type: u32,
    pub npc_flags: i32,
    pub unit_flags: i32,
    pub civilian: bool,
    pub health: i32,
    pub melee_damage: i32,
    pub loot_id: i32,
    pub gossip_menu_id: u32,
    pub vendor_template_id: u32,
    pub skinning_loot_id: i32,
    pub pickpocket_loot_id: i32,
}

#[derive(Clone, Debug)]
pub struct SpawnRow {
    pub guid: u32,
    pub entry: u32,
    pub map_id: u32,
    pub position: Position,
    pub respawn_secs: u32,
}

impl Catalog {
    pub fn creatures_for_map(&self, map_id: u32) -> Vec<Creature> {
        let Some(spawns) = self.spawns.get(&map_id) else {
            return Vec::new();
        };
        spawns
            .iter()
            .filter_map(|spawn| self.creature_from_spawn(spawn))
            .collect()
    }

    pub fn creature_type(&self, entry: u32) -> Option<&CreatureTypeRow> {
        self.types.get(&entry)
    }

    pub fn lookup_creatures(&self, query: &str) -> Vec<&CreatureTypeRow> {
        super::lookup_named(self.types.values(), query, |creature| {
            creature.name.as_str()
        })
    }

    pub fn spawn_creature(&self, entry: u32, guid: u32, position: Position) -> Option<Creature> {
        self.creature_from_spawn(&SpawnRow {
            guid,
            entry,
            map_id: 0,
            position,
            respawn_secs: 120,
        })
    }

    pub(crate) fn creature_from_spawn(&self, spawn: &SpawnRow) -> Option<Creature> {
        let kind = self.types.get(&spawn.entry)?;
        let gossip = self.gossip(kind.gossip_menu_id);
        let service = kind.npc_flags & SERVICE_NPC_FLAGS != 0;
        let unattackable = kind.unit_flags & UNIT_FLAGS_NO_PLAYER_AGGRO != 0;
        let critter = kind.creature_type == CREATURE_TYPE_CRITTER;
        let can_fight = !kind.civilian && !service && !unattackable && !critter;
        let mut faction = kind.faction;
        // City templates (Stormwind, Darnassus, …) stay on the wire. Neutral
        // wildlife (Creature 7 / 189) is rewritten to a client-hostile template.
        if can_fight {
            faction = self.factions.ensure_hostile_to_players(faction);
        }
        let hostile = can_fight && self.factions.hostile_to_players(faction);
        Some(Creature {
            guid: unit_guid(spawn.guid, spawn.entry),
            entry: spawn.entry,
            name: kind.name.clone(),
            sub_name: kind.sub_name.clone(),
            display_id: kind.display_id,
            faction,
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
            lootable: false,
            gossip,
            loot_id: kind.loot_id,
            respawn_secs: spawn.respawn_secs.max(5),
            melee_damage: kind.melee_damage,
        })
    }

    pub(crate) async fn load_types(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT entry, name, sub_name, min_level, max_level, display_id, faction::text AS faction, family,
                    creature_type, npc_flags, unit_flags, civilian, health, melee_damage,
                    loot_id, gossip_menu_id, vendor_template_id, skinning_loot_id, pickpocket_loot_id
             FROM creature_types",
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
                    max_level: i32::from(row.get::<i16, _>(4)),
                    display_id: row.get(5),
                    faction: faction.as_protocol() as i32,
                    family: row.get::<i16, _>(7) as u8,
                    creature_type: row.get::<i32, _>(8) as u32,
                    npc_flags: row.get(9),
                    unit_flags: row.get(10),
                    civilian: row.get(11),
                    health: row.get(12),
                    melee_damage: row.get(13),
                    loot_id: row.get(14),
                    gossip_menu_id: row.get::<i32, _>(15) as u32,
                    vendor_template_id: row.get::<i32, _>(16) as u32,
                    skinning_loot_id: row.get(17),
                    pickpocket_loot_id: row.get(18),
                },
            );
        }
        if skipped > 0 {
            tracing::warn!(skipped, "skipped creature_types with unknown faction");
        }
        Ok(())
    }

    pub(crate) async fn load_spawns(&mut self, pool: &PgPool) -> anyhow::Result<()> {
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
}

fn family_from_u32(value: u32) -> CreatureFamily {
    CreatureFamily::try_from(value as u8).unwrap_or(CreatureFamily::None)
}

#[cfg(test)]
mod tests {
    use wow_shared::Position;

    use super::*;
    use crate::catalog::Catalog;
    use crate::creature::NPC_FLAG_GOSSIP;

    fn type_row(entry: u32, faction: i32, npc_flags: i32, civilian: bool) -> CreatureTypeRow {
        type_row_flags(entry, faction, npc_flags, 0, civilian)
    }

    fn type_row_flags(
        entry: u32,
        faction: i32,
        npc_flags: i32,
        unit_flags: i32,
        civilian: bool,
    ) -> CreatureTypeRow {
        CreatureTypeRow {
            entry,
            name: "Test".into(),
            sub_name: String::new(),
            level: 1,
            max_level: 2,
            display_id: 1,
            faction,
            family: 1,
            creature_type: 1,
            npc_flags,
            unit_flags,
            civilian,
            health: 45,
            melee_damage: 4,
            loot_id: 0,
            gossip_menu_id: 0,
            vendor_template_id: 0,
            skinning_loot_id: 0,
            pickpocket_loot_id: 0,
        }
    }

    #[test]
    fn hostile_wolf_template_32_is_promoted_to_38() {
        let mut catalog = Catalog::empty();
        catalog.types.insert(299, type_row(299, 32, 0, false));
        let creature = catalog
            .spawn_creature(299, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(creature.hostile);
        assert_eq!(creature.faction, 38);
    }

    #[test]
    fn stormwind_guard_without_gossip_stays_friendly() {
        let mut catalog = Catalog::empty();
        catalog.types.insert(1423, type_row(1423, 11, 0, false));
        let creature = catalog
            .spawn_creature(1423, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(!creature.hostile);
        assert_eq!(creature.faction, 11);
    }

    #[test]
    fn stormwind_guard_with_gossip_stays_friendly() {
        let mut catalog = Catalog::empty();
        catalog
            .types
            .insert(1423, type_row(1423, 11, NPC_FLAG_GOSSIP, false));
        let creature = catalog
            .spawn_creature(1423, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(!creature.hostile);
        assert_eq!(creature.faction, 11);
    }

    #[test]
    fn northshire_guard_pvp_flag_stays_stormwind() {
        let mut catalog = Catalog::empty();
        catalog
            .types
            .insert(1642, type_row_flags(1642, 11, 0, 4096, false));
        let creature = catalog
            .spawn_creature(1642, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(!creature.hostile);
        assert_eq!(creature.faction, 11);
    }

    #[test]
    fn passive_unit_flags_do_not_aggro() {
        let mut catalog = Catalog::empty();
        catalog
            .types
            .insert(1, type_row_flags(1, 14, 0, 0x0200, false));
        let creature = catalog
            .spawn_creature(1, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(!creature.hostile);
        assert_eq!(creature.faction, 14);
    }

    #[test]
    fn nightsaber_creature_7_becomes_attackable() {
        let mut catalog = Catalog::empty();
        catalog.types.insert(2031, type_row(2031, 7, 0, false));
        let creature = catalog
            .spawn_creature(2031, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(creature.hostile);
        assert_eq!(creature.faction, 14);
    }

    #[test]
    fn grell_creature_189_becomes_attackable() {
        let mut catalog = Catalog::empty();
        catalog.types.insert(1988, type_row(1988, 189, 0, false));
        let creature = catalog
            .spawn_creature(1988, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(creature.hostile);
        assert_eq!(creature.faction, 14);
    }

    #[test]
    fn darnassus_sentinel_stays_friendly() {
        let mut catalog = Catalog::empty();
        catalog
            .types
            .insert(12160, type_row_flags(12160, 79, 0, 4096, false));
        let creature = catalog
            .spawn_creature(12160, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(!creature.hostile);
        assert_eq!(creature.faction, 79);
    }

    #[test]
    fn prey_critter_is_not_hostile() {
        let mut catalog = Catalog::empty();
        let mut row = type_row(721, 31, 0, false);
        row.creature_type = CREATURE_TYPE_CRITTER;
        catalog.types.insert(721, row);
        let creature = catalog
            .spawn_creature(721, 1, Position::NORTHSHIRE)
            .expect("spawn");
        assert!(!creature.hostile);
        assert_eq!(creature.faction, 31);
    }
}
