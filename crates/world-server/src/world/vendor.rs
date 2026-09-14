use crate::catalog::{Catalog, VendorListing};

use super::visibility::in_range;
use super::{GOSSIP_RANGE, Inner, PlayerMailbox, World, WorldEvent, broadcast};

pub(crate) const LOOT_RANGE: f32 = 20.0;
const LOOT_ERROR_DIDNT_KILL: u8 = 0;
const LOOT_ERROR_TOO_FAR: u8 = 4;

impl World {
    pub fn list_vendor(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        };
        let Some(creature) = inner.creatures.get(&npc) else {
            return;
        };
        if creature.creature.npc_flags & crate::creature::NPC_FLAG_VENDOR == 0 {
            return;
        }
        if !in_range(
            presence.player.position,
            creature.creature.position,
            GOSSIP_RANGE,
        ) {
            return;
        }
        let entry = creature.creature.entry;
        let listings = self
            .catalog
            .as_ref()
            .map(|catalog| catalog.vendor_listings(entry))
            .unwrap_or_default();
        let items = listings
            .into_iter()
            .filter_map(|listing| vendor_offer(self.catalog.as_deref(), listing))
            .collect();
        from.send(WorldEvent::GossipClosed);
        from.send(WorldEvent::VendorOpened { npc, items });
    }

    pub fn buy_item(&self, from: &PlayerMailbox, player: u64, npc: u64, item_id: u32, amount: u32) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let Some(creature) = inner.creatures.get(&npc) else {
            return;
        };
        if creature.creature.npc_flags & crate::creature::NPC_FLAG_VENDOR == 0 {
            return;
        }
        let entry = creature.creature.entry;
        let Some(catalog) = self.catalog.as_ref() else {
            return;
        };
        let listings = catalog.vendor_listings(entry);
        if !listings.iter().any(|listing| listing.item_id == item_id) {
            return;
        }
        let Some(item) = catalog.item(item_id) else {
            return;
        };
        let amount = amount.max(1);
        let price = item.buy_price.saturating_mul(amount);
        let stackable = item.stackable;
        let presence = inner.players.get_mut(&player).expect("player");
        if presence.player.copper < price {
            return;
        }
        if !presence.player.add_item(item_id, amount, stackable) {
            return;
        }
        presence.player.copper -= price;
        let copper = presence.player.copper;
        from.send(WorldEvent::MoneyChanged {
            guid: player,
            copper,
        });
    }

    pub fn open_loot(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            tracing::info!(player, npc, "loot ignored: player not on map");
            return;
        };
        if !presence.mailbox.same_channel(from) {
            tracing::info!(player, npc, "loot ignored: mailbox mismatch");
            return;
        }
        let position = presence.player.position;
        let Some(creature) = inner.creatures.get(&npc) else {
            tracing::info!(player, npc, "loot failed: unknown guid");
            from.send(WorldEvent::LootFailed {
                guid: npc,
                error: LOOT_ERROR_DIDNT_KILL,
            });
            return;
        };
        if !creature.creature.dead {
            tracing::info!(player, npc, "loot failed: creature is alive");
            from.send(WorldEvent::LootFailed {
                guid: npc,
                error: LOOT_ERROR_DIDNT_KILL,
            });
            return;
        }
        if !in_range(position, creature.creature.position, LOOT_RANGE) {
            tracing::info!(player, npc, "loot failed: too far");
            from.send(WorldEvent::LootFailed {
                guid: npc,
                error: LOOT_ERROR_TOO_FAR,
            });
            return;
        }
        if let Some(event) = offer_creature_loot(&mut inner, self.catalog.as_deref(), player, npc) {
            from.send(event);
        }
    }

    pub fn take_loot(&self, from: &PlayerMailbox, player: u64, index: u8) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        }
        let Some(npc) = presence.looting else {
            return;
        };
        let Some(drops) = inner.loot.get(&npc).cloned() else {
            return;
        };
        if index as usize >= drops.len() {
            return;
        }
        let (item_id, count) = drops[index as usize];
        if item_id == 0 {
            return;
        }
        let stackable = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.item(item_id))
            .map(|item| item.stackable)
            .unwrap_or(1);
        let presence = inner.players.get_mut(&player).expect("player");
        if !presence.player.add_item(item_id, count, stackable) {
            return;
        }
        if let Some(drops) = inner.loot.get_mut(&npc) {
            drops[index as usize] = (0, 0);
        }
        let remaining = inner
            .loot
            .get(&npc)
            .is_some_and(|drops| drops.iter().any(|(item_id, _)| *item_id != 0));
        let (health, max_health) = inner
            .creatures
            .get(&npc)
            .map(|state| (state.creature.health, state.creature.max_health))
            .unwrap_or((0, 0));
        if let Some(state) = inner.creatures.get_mut(&npc) {
            state.creature.lootable = remaining;
        }
        from.send(WorldEvent::LootTaken { index });
        if !remaining {
            broadcast(
                &inner,
                WorldEvent::MeleeHit(super::MeleeHit {
                    attacker: player,
                    victim: npc,
                    damage: 0,
                    victim_health: health,
                    victim_max_health: max_health,
                    victim_dead: true,
                    lootable: false,
                }),
            );
        }
    }

    pub fn close_loot(&self, from: &PlayerMailbox, player: u64) {
        let mut inner = self.inner.lock().expect("world mutex");
        if let Some(presence) = inner.players.get_mut(&player) {
            if presence.mailbox.same_channel(from) {
                let guid = presence.looting.take().unwrap_or(0);
                from.send(WorldEvent::LootClosed { guid });
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VendorOffer {
    pub item_id: u32,
    pub display_id: u32,
    pub max_items: u32,
    pub price: u32,
    pub max_durability: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LootOffer {
    pub index: u8,
    pub item_id: u32,
    pub count: u32,
    pub display_id: u32,
}

pub(crate) fn fill_creature_loot(
    inner: &mut Inner,
    catalog: Option<&Catalog>,
    npc: u64,
) {
    if inner.loot.contains_key(&npc) {
        return;
    }
    let Some(state) = inner.creatures.get(&npc) else {
        return;
    };
    let loot_id = state.creature.loot_id;
    let drops = catalog
        .map(|catalog| catalog.roll_loot(loot_id))
        .unwrap_or_default();
    let lootable = drops.iter().any(|(item_id, _)| *item_id != 0);
    tracing::info!(
        npc,
        loot_id,
        items = drops.len(),
        lootable,
        "rolled creature loot"
    );
    if let Some(state) = inner.creatures.get_mut(&npc) {
        state.creature.lootable = lootable;
    }
    inner.loot.insert(npc, drops);
}

pub(crate) fn offer_creature_loot(
    inner: &mut Inner,
    catalog: Option<&Catalog>,
    player: u64,
    npc: u64,
) -> Option<WorldEvent> {
    fill_creature_loot(inner, catalog, npc);
    let items = inner
        .loot
        .get(&npc)
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, (item_id, _))| *item_id != 0)
        .map(|(index, (item_id, count))| LootOffer {
            index: index as u8,
            item_id: *item_id,
            count: *count,
            display_id: catalog
                .and_then(|catalog| catalog.item(*item_id))
                .map(|item| item.display_id)
                .unwrap_or(0),
        })
        .take(16)
        .collect::<Vec<_>>();
    if items.is_empty() {
        tracing::info!(player, npc, "corpse has no loot");
        return None;
    }
    if let Some(presence) = inner.players.get_mut(&player) {
        presence.looting = Some(npc);
    }
    tracing::info!(player, npc, items = items.len(), "opened creature loot");
    Some(WorldEvent::LootOpened {
        guid: npc,
        gold: 0,
        items,
    })
}

fn vendor_offer(catalog: Option<&Catalog>, listing: VendorListing) -> Option<VendorOffer> {
    let item = catalog?.item(listing.item_id)?;
    Some(VendorOffer {
        item_id: listing.item_id,
        display_id: item.display_id,
        max_items: if listing.maxcount <= 0 {
            u32::MAX
        } else {
            listing.maxcount as u32
        },
        price: item.buy_price,
        max_durability: item.max_durability,
    })
}
