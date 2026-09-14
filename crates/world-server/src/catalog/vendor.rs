use sqlx::PgPool;

use super::Catalog;

#[derive(Clone, Debug)]
pub struct VendorListing {
    pub item_id: u32,
    pub slot: i16,
    pub maxcount: i16,
}

impl Catalog {
    pub fn vendor_listings(&self, creature_entry: u32) -> Vec<VendorListing> {
        let mut listings = self
            .vendors
            .get(&creature_entry)
            .cloned()
            .unwrap_or_default();
        if let Some(kind) = self.types.get(&creature_entry)
            && kind.vendor_template_id != 0
            && let Some(template) = self.vendor_templates.get(&kind.vendor_template_id)
        {
            listings.extend(template.iter().cloned());
        }
        listings.sort_by_key(|listing| listing.slot);
        listings
    }

    pub(crate) async fn load_vendors(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let vendors = sqlx::query_as::<_, (i32, i32, i16, i16)>(
            "SELECT creature_entry, item_id, slot, maxcount FROM creature_vendors",
        )
        .fetch_all(pool)
        .await?;
        for row in vendors {
            self.vendors
                .entry(row.0 as u32)
                .or_default()
                .push(VendorListing {
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
}

#[cfg(test)]
mod tests {
    use crate::creature::NPC_FLAG_VENDOR;

    use super::*;

    #[test]
    fn vendor_listings_append_the_shared_template() {
        let mut catalog = Catalog::empty();
        catalog.types.insert(
            6,
            crate::catalog::CreatureTypeRow {
                entry: 6,
                name: "Kobold".into(),
                sub_name: String::new(),
                level: 1,
                max_level: 1,
                display_id: 1,
                faction: 1,
                family: 0,
                creature_type: 7,
                npc_flags: NPC_FLAG_VENDOR,
                unit_flags: 0,
                civilian: true,
                health: 30,
                melee_damage: 0,
                loot_id: 0,
                gossip_menu_id: 0,
                vendor_template_id: 9,
                skinning_loot_id: 0,
                pickpocket_loot_id: 0,
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
