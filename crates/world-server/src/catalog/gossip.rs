use std::collections::HashSet;

use sqlx::{PgPool, Row};
use wow_shared::{GossipOptionIcon, GossipOptionKind};

use crate::creature::{Gossip, GossipAction, GossipMenu, GossipOption};

use super::Catalog;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NpcTextPage {
    pub probability: f32,
    pub text: String,
    pub text_female: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BroadcastTextRow {
    pub id: u32,
    pub text: String,
    pub text_female: String,
    pub chat_type: i16,
    pub language_id: i32,
    pub sound_id: i32,
    pub emote_id: i32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct GossipTextBroadcastRow {
    pub text_id: u32,
    pub probability: [f32; 8],
    pub broadcast_text_id: [u32; 8],
}

impl Catalog {
    pub fn gossip_text(&self, text_id: u32) -> Option<String> {
        let pages = self.npc_text_pages(text_id);
        pages
            .iter()
            .find(|page| !page.text.is_empty() || !page.text_female.is_empty())
            .map(|page| {
                if !page.text.is_empty() {
                    page.text.clone()
                } else {
                    page.text_female.clone()
                }
            })
    }

    pub fn npc_text_pages(&self, text_id: u32) -> [NpcTextPage; 8] {
        if let Some(mapped) = self.gossip_text_broadcasts.get(&text_id) {
            return std::array::from_fn(|i| {
                let id = mapped.broadcast_text_id[i];
                let (text, text_female) = if id == 0 {
                    (String::new(), String::new())
                } else {
                    self.broadcast_texts
                        .get(&id)
                        .map(|row| (row.text.clone(), row.text_female.clone()))
                        .unwrap_or_default()
                };
                NpcTextPage {
                    probability: mapped.probability[i] / 100.0,
                    text,
                    text_female,
                }
            });
        }
        let mut pages = std::array::from_fn(|_| NpcTextPage::default());
        if let Some(text) = self.gossip_texts.get(&text_id) {
            pages[0] = NpcTextPage {
                probability: 1.0,
                text: text.clone(),
                text_female: text.clone(),
            };
        }
        pages
    }

    pub(crate) fn gossip(&self, menu_id: u32) -> Option<Gossip> {
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
            let text = self.gossip_text(text_id).unwrap_or_default();
            let mut options = Vec::new();
            if let Some(raw) = self.gossip_options.get(&id) {
                for (option_id, option, kind) in raw {
                    let mut option = option.clone();
                    option.id = *option_id;
                    if *kind == GossipOptionKind::Vendor {
                        option.action = GossipAction::OpenVendor;
                    } else if *kind == GossipOptionKind::QuestGiver {
                        option.action = GossipAction::OpenQuestGiver;
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

    pub(crate) async fn load_gossip(&mut self, pool: &PgPool) -> anyhow::Result<()> {
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
        let mapped = sqlx::query(
            "SELECT text_id,
                    probability_0, probability_1, probability_2, probability_3,
                    probability_4, probability_5, probability_6, probability_7,
                    broadcast_text_id_0, broadcast_text_id_1, broadcast_text_id_2,
                    broadcast_text_id_3, broadcast_text_id_4, broadcast_text_id_5,
                    broadcast_text_id_6, broadcast_text_id_7
             FROM gossip_text_broadcasts",
        )
        .fetch_all(pool)
        .await?;
        for row in mapped {
            let text_id: i32 = row.get("text_id");
            let mut probability = [0.0; 8];
            let mut broadcast_text_id = [0; 8];
            for i in 0..8 {
                let prob: f32 = row.get(format!("probability_{i}").as_str());
                let id: i32 = row.get(format!("broadcast_text_id_{i}").as_str());
                probability[i] = prob;
                broadcast_text_id[i] = id as u32;
            }
            self.gossip_text_broadcasts.insert(
                text_id as u32,
                GossipTextBroadcastRow {
                    text_id: text_id as u32,
                    probability,
                    broadcast_text_id,
                },
            );
        }
        let options = sqlx::query(
            "SELECT menu_id, option_id, icon::text AS icon, text, option_kind::text AS option_kind, action_menu_id
             FROM gossip_options",
        )
        .fetch_all(pool)
        .await?;
        let mut skipped = 0u64;
        for row in options {
            let kind_label: String = row.get("option_kind");
            let icon_label: String = row.get("icon");
            let Some(kind) = GossipOptionKind::from_str(&kind_label) else {
                skipped += 1;
                continue;
            };
            let Some(icon) = GossipOptionIcon::from_str(&icon_label) else {
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
                        icon,
                        kind,
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

    pub(crate) async fn load_broadcast_texts(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT id, text, text_female, chat_type, language_id, sound_id, emote_id
             FROM broadcast_texts",
        )
        .fetch_all(pool)
        .await?;
        for row in rows {
            let id: i32 = row.get("id");
            self.broadcast_texts.insert(
                id as u32,
                BroadcastTextRow {
                    id: id as u32,
                    text: row.get("text"),
                    text_female: row.get("text_female"),
                    chat_type: row.get("chat_type"),
                    language_id: row.get("language_id"),
                    sound_id: row.get("sound_id"),
                    emote_id: row.get("emote_id"),
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use wow_shared::{GossipOptionIcon, GossipOptionKind};

    use crate::creature::{GossipAction, GossipOption};

    use super::*;

    fn broadcast_row(id: u32, text: &str) -> BroadcastTextRow {
        BroadcastTextRow {
            id,
            text: text.into(),
            text_female: String::new(),
            chat_type: 0,
            language_id: 0,
            sound_id: 0,
            emote_id: 0,
        }
    }

    #[test]
    fn gossip_text_prefers_broadcast_mapping() {
        let mut catalog = Catalog::empty();
        catalog.gossip_texts.insert(10, "from npc_text".into());
        catalog
            .broadcast_texts
            .insert(99, broadcast_row(99, "from broadcast"));
        catalog.gossip_text_broadcasts.insert(
            10,
            GossipTextBroadcastRow {
                text_id: 10,
                probability: [100.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                broadcast_text_id: [99, 0, 0, 0, 0, 0, 0, 0],
            },
        );
        assert_eq!(catalog.gossip_text(10).as_deref(), Some("from broadcast"));
        let pages = catalog.npc_text_pages(10);
        assert_eq!(pages[0].probability, 1.0);
        assert_eq!(pages[0].text, "from broadcast");
        assert!(
            pages[1..]
                .iter()
                .all(|page| page.probability == 0.0 && page.text.is_empty())
        );
    }

    #[test]
    fn npc_text_pages_weight_eight_slots() {
        let mut catalog = Catalog::empty();
        catalog.broadcast_texts.insert(1, broadcast_row(1, "a"));
        catalog.broadcast_texts.insert(2, broadcast_row(2, "b"));
        catalog.gossip_text_broadcasts.insert(
            10,
            GossipTextBroadcastRow {
                text_id: 10,
                probability: [50.0, 50.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                broadcast_text_id: [1, 2, 0, 0, 0, 0, 0, 0],
            },
        );
        let pages = catalog.npc_text_pages(10);
        assert_eq!(pages[0].probability, 0.5);
        assert_eq!(pages[0].text, "a");
        assert_eq!(pages[1].probability, 0.5);
        assert_eq!(pages[1].text, "b");
    }

    #[test]
    fn gossip_text_falls_back_to_npc_text() {
        let mut catalog = Catalog::empty();
        catalog.gossip_texts.insert(10, "from npc_text".into());
        assert_eq!(catalog.gossip_text(10).as_deref(), Some("from npc_text"));
        assert!(catalog.gossip_text(11).is_none());
    }

    #[test]
    fn gossip_maps_questgiver_kind_to_open_questgiver() {
        let mut catalog = Catalog::empty();
        catalog.gossip_menu_text.insert(1, vec![10]);
        catalog.gossip_texts.insert(10, "hello".into());
        catalog.gossip_options.insert(
            1,
            vec![(
                0,
                GossipOption {
                    id: 0,
                    text: "quests".into(),
                    icon: GossipOptionIcon::Chat,
                    kind: GossipOptionKind::QuestGiver,
                    action: GossipAction::Close,
                },
                GossipOptionKind::QuestGiver,
            )],
        );
        let gossip = catalog.gossip(1).expect("menu");
        assert_eq!(
            gossip.greeting().options[0].action,
            GossipAction::OpenQuestGiver
        );
    }
}
