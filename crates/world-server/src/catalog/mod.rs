mod creature;
mod gameobject;
mod gossip;
mod item;
mod loot;
mod quest;
mod vendor;

use std::collections::HashMap;
use std::sync::Arc;

use client_data::FactionTemplates;
use sqlx::PgPool;
use wow_shared::GossipOptionKind;

use crate::creature::GossipOption;

pub use creature::{CreatureTypeRow, SpawnRow};
pub use gameobject::{GameObjectSpawnRow, GameObjectTypeRow};
pub use gossip::{BroadcastTextRow, GossipTextBroadcastRow, NpcTextPage};
pub use item::ItemRow;
pub(crate) use loot::LootRow;
pub use quest::{QuestRow, QuestgiverKind};
pub use vendor::VendorListing;

#[derive(Clone, Debug)]
pub struct Catalog {
    pub(crate) types: HashMap<u32, CreatureTypeRow>,
    pub(crate) spawns: HashMap<u32, Vec<SpawnRow>>,
    pub(crate) items: HashMap<u32, ItemRow>,
    pub(crate) vendors: HashMap<u32, Vec<VendorListing>>,
    pub(crate) vendor_templates: HashMap<u32, Vec<VendorListing>>,
    pub(crate) loot: HashMap<i32, Vec<LootRow>>,
    pub(crate) loot_refs: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) gameobject_loot: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) item_loot: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) fishing_loot: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) skinning_loot: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) pickpocket_loot: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) disenchant_loot: HashMap<i32, Vec<LootRow>>,
    #[allow(dead_code)]
    pub(crate) mail_loot: HashMap<i32, Vec<LootRow>>,
    pub(crate) gossip_menu_text: HashMap<u32, Vec<u32>>,
    pub(crate) gossip_options: HashMap<u32, Vec<(u32, GossipOption, GossipOptionKind)>>,
    pub(crate) gossip_texts: HashMap<u32, String>,
    pub(crate) gossip_text_broadcasts: HashMap<u32, GossipTextBroadcastRow>,
    pub(crate) quests: HashMap<u32, QuestRow>,
    pub(crate) creature_quest_starts: HashMap<u32, Vec<u32>>,
    #[allow(dead_code)]
    pub(crate) creature_quest_ends: HashMap<u32, Vec<u32>>,
    #[allow(dead_code)]
    pub(crate) gameobject_quest_starts: HashMap<u32, Vec<u32>>,
    #[allow(dead_code)]
    pub(crate) gameobject_quest_ends: HashMap<u32, Vec<u32>>,
    #[allow(dead_code)]
    pub(crate) areatrigger_quest_ends: HashMap<u32, Vec<u32>>,
    #[allow(dead_code)]
    pub(crate) questgiver_greetings: HashMap<(QuestgiverKind, u32), String>,
    pub(crate) broadcast_texts: HashMap<u32, BroadcastTextRow>,
    pub(crate) gameobject_types: HashMap<u32, GameObjectTypeRow>,
    pub(crate) gameobject_spawns: HashMap<u32, Vec<GameObjectSpawnRow>>,
    pub(crate) factions: FactionTemplates,
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
            gameobject_loot: HashMap::new(),
            item_loot: HashMap::new(),
            fishing_loot: HashMap::new(),
            skinning_loot: HashMap::new(),
            pickpocket_loot: HashMap::new(),
            disenchant_loot: HashMap::new(),
            mail_loot: HashMap::new(),
            gossip_menu_text: HashMap::new(),
            gossip_options: HashMap::new(),
            gossip_texts: HashMap::new(),
            gossip_text_broadcasts: HashMap::new(),
            quests: HashMap::new(),
            creature_quest_starts: HashMap::new(),
            creature_quest_ends: HashMap::new(),
            gameobject_quest_starts: HashMap::new(),
            gameobject_quest_ends: HashMap::new(),
            areatrigger_quest_ends: HashMap::new(),
            questgiver_greetings: HashMap::new(),
            broadcast_texts: HashMap::new(),
            gameobject_types: HashMap::new(),
            gameobject_spawns: HashMap::new(),
            factions: FactionTemplates::builtin(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty() || self.spawns.is_empty()
    }

    pub async fn load(pool: &PgPool, factions: FactionTemplates) -> anyhow::Result<Arc<Self>> {
        let mut catalog = Self::empty();
        catalog.factions = factions;
        catalog.load_types(pool).await?;
        catalog.load_spawns(pool).await?;
        catalog.load_items(pool).await?;
        catalog.load_vendors(pool).await?;
        catalog.load_loot(pool).await?;
        catalog.load_gossip(pool).await?;
        catalog.load_quests(pool).await?;
        catalog.load_broadcast_texts(pool).await?;
        catalog.load_gameobjects(pool).await?;
        tracing::info!(
            types = catalog.types.len(),
            spawns = catalog.spawns.values().map(Vec::len).sum::<usize>(),
            items = catalog.items.len(),
            creature_loot = Self::loot_row_count(&catalog.loot),
            loot_references = Self::loot_row_count(&catalog.loot_refs),
            gameobject_loot = Self::loot_row_count(&catalog.gameobject_loot),
            item_loot = Self::loot_row_count(&catalog.item_loot),
            fishing_loot = Self::loot_row_count(&catalog.fishing_loot),
            skinning_loot = Self::loot_row_count(&catalog.skinning_loot),
            pickpocket_loot = Self::loot_row_count(&catalog.pickpocket_loot),
            disenchant_loot = Self::loot_row_count(&catalog.disenchant_loot),
            mail_loot = Self::loot_row_count(&catalog.mail_loot),
            quests = catalog.quests.len(),
            broadcast_texts = catalog.broadcast_texts.len(),
            gossip_text_broadcasts = catalog.gossip_text_broadcasts.len(),
            gameobject_types = catalog.gameobject_types.len(),
            gameobject_spawns = catalog
                .gameobject_spawns
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            faction_templates = catalog.factions.len(),
            faction_source = catalog
                .factions
                .source_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "builtin".into()),
            "loaded world catalog"
        );
        catalog.log_unhandled_summary();
        Ok(Arc::new(catalog))
    }

    fn log_unhandled_summary(&self) {
        let gossip_kinds = self
            .gossip_options
            .values()
            .flatten()
            .filter(|(_, _, kind)| !kind.is_handled())
            .count();
        let quests = self
            .quests
            .values()
            .filter(|quest| crate::quest::has_unhandled_fields(quest))
            .count();
        let npc_flags = self
            .types
            .values()
            .filter(|kind| crate::creature::unhandled_npc_flags(kind.npc_flags) != 0)
            .count();
        if gossip_kinds == 0 && quests == 0 && npc_flags == 0 {
            return;
        }
        tracing::warn!(
            gossip_kinds,
            quests,
            npc_flags,
            "catalog fields the world server does not handle yet"
        );
    }
}

pub(crate) fn lookup_named<'a, T>(
    values: impl Iterator<Item = &'a T>,
    query: &str,
    name: impl Fn(&T) -> &str,
) -> Vec<&'a T> {
    let query = query.to_ascii_lowercase();
    let mut matches: Vec<_> = values
        .filter(|value| name(value).to_ascii_lowercase().contains(&query))
        .collect();
    matches.sort_by_key(|value| name(value).to_ascii_lowercase());
    matches.truncate(20);
    matches
}
