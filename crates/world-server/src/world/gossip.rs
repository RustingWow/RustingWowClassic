use crate::catalog::{Catalog, NpcTextPage};
use crate::creature::{Gossip, GossipAction, GossipMenu, unhandled_npc_flags};
use crate::quest;

use super::quest::{quest_items_for, send_quest_details};
use super::visibility::in_range;
use super::{CreatureState, GOSSIP_RANGE, Inner, PlayerMailbox, Presence, World, WorldEvent};

impl World {
    pub fn open_gossip(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let catalog = self.catalog.clone();
        let inner = self.inner.lock().expect("world mutex");
        let Some((presence, creature)) = dialog_npc(&inner, from, player, npc) else {
            return;
        };
        if unhandled_npc_flags(creature.creature.npc_flags) != 0 {
            tracing::info!(
                npc,
                npc_flags = unhandled_npc_flags(creature.creature.npc_flags),
                "unhandled npc flags"
            );
        }
        let menu = creature
            .creature
            .gossip
            .as_ref()
            .map(|gossip| gossip.greeting().clone())
            .unwrap_or_else(|| GossipMenu {
                text_id: 0,
                text: creature.creature.name.clone(),
                options: Vec::new(),
            });
        let pages = pages_for(catalog.as_deref(), menu.text_id, &menu.text);
        let quests = quest_items_for(
            catalog.as_deref(),
            &presence.player,
            creature.creature.entry,
        );
        tracing::info!(
            npc,
            entry = creature.creature.entry,
            options = menu.options.len(),
            quests = quests.len(),
            "gossip opened"
        );
        from.send(WorldEvent::GossipOpened {
            npc,
            menu,
            quests,
            pages,
        });
    }

    pub fn select_gossip_option(
        &self,
        from: &PlayerMailbox,
        player: u64,
        npc: u64,
        option_id: u32,
    ) {
        let catalog = self.catalog.clone();
        let inner = self.inner.lock().expect("world mutex");
        let Some(gossip) = gossip_for(&inner, from, player, npc) else {
            tracing::info!(
                npc,
                option_id,
                "gossip select without menu; trying as quest"
            );
            drop(inner);
            self.query_quest_details(from, player, npc, option_id);
            return;
        };
        let option = gossip.option(option_id).cloned().or_else(|| {
            gossip
                .menus
                .iter()
                .flat_map(|menu| menu.options.iter())
                .nth(option_id as usize)
                .cloned()
        });
        let Some(option) = option else {
            tracing::info!(npc, option_id, "unknown gossip option; trying as quest");
            drop(inner);
            self.query_quest_details(from, player, npc, option_id);
            return;
        };
        let action = option.action;
        let kind = option.kind;
        let text = option.text.clone();
        let next_menu = match action {
            GossipAction::ShowMenu { text_id } => gossip.menu(text_id).cloned(),
            _ => None,
        };
        match action {
            GossipAction::Close => {
                if !kind.is_handled() {
                    tracing::info!(
                        npc,
                        option_id,
                        kind = kind.as_str(),
                        text = text.as_str(),
                        "unhandled gossip option kind"
                    );
                }
                from.send(WorldEvent::GossipClosed);
            }
            GossipAction::ShowMenu { .. } => {
                let Some(menu) = next_menu else {
                    return;
                };
                let pages = pages_for(catalog.as_deref(), menu.text_id, &menu.text);
                let quests = inner
                    .players
                    .get(&player)
                    .zip(inner.creatures.get(&npc))
                    .map(|(presence, creature)| {
                        quest_items_for(
                            catalog.as_deref(),
                            &presence.player,
                            creature.creature.entry,
                        )
                    })
                    .unwrap_or_default();
                from.send(WorldEvent::GossipOpened {
                    npc,
                    menu,
                    quests,
                    pages,
                });
            }
            GossipAction::OpenVendor => {
                drop(inner);
                self.list_vendor(from, player, npc);
            }
            GossipAction::OpenQuestGiver => {
                let Some((presence, creature)) = dialog_npc(&inner, from, player, npc) else {
                    return;
                };
                let fallback = creature
                    .creature
                    .gossip
                    .as_ref()
                    .map(|gossip| gossip.greeting().text.clone())
                    .unwrap_or_default();
                let npc_entry = creature.creature.entry;
                let title = catalog
                    .as_ref()
                    .map(|catalog| quest::greeting_text(catalog, npc_entry, &fallback))
                    .unwrap_or(fallback);
                let listed = catalog
                    .as_ref()
                    .map(|catalog| quest::offered_quests(catalog, &presence.player, npc_entry))
                    .unwrap_or_default();
                match listed.as_slice() {
                    [(quest, false)] => send_quest_details(from, npc, (*quest).clone()),
                    _ => {
                        from.send(WorldEvent::GossipClosed);
                        match listed.as_slice() {
                            [] => {}
                            [(quest, true)] => from.send(WorldEvent::QuestOfferReward {
                                npc,
                                quest: (*quest).clone(),
                            }),
                            _ => {
                                let quests = listed
                                    .into_iter()
                                    .map(|(quest, complete)| crate::quest::GossipQuestItem {
                                        quest_id: quest.entry,
                                        icon: if complete {
                                            quest::QUEST_ICON_COMPLETE
                                        } else {
                                            quest::QUEST_ICON_AVAILABLE
                                        },
                                        level: quest.quest_level,
                                        title: quest.title.clone(),
                                    })
                                    .collect();
                                from.send(WorldEvent::QuestList { npc, title, quests });
                            }
                        }
                    }
                }
            }
        }
    }
}

fn gossip_for<'a>(
    inner: &'a Inner,
    from: &PlayerMailbox,
    player: u64,
    npc: u64,
) -> Option<&'a Gossip> {
    let (_, creature) = dialog_npc(inner, from, player, npc)?;
    if !creature.creature.can_gossip() {
        return None;
    }
    creature.creature.gossip.as_ref()
}

pub(crate) fn dialog_npc<'a>(
    inner: &'a Inner,
    from: &PlayerMailbox,
    player: u64,
    npc: u64,
) -> Option<(&'a Presence, &'a CreatureState)> {
    let presence = inner.players.get(&player)?;
    if !presence.mailbox.same_channel(from) {
        return None;
    }
    let creature = inner.creatures.get(&npc)?;
    if !creature.creature.can_questgiver() && !creature.creature.can_gossip() {
        return None;
    }
    if !in_range(
        presence.player.position,
        creature.creature.position,
        GOSSIP_RANGE,
    ) {
        return None;
    }
    Some((presence, creature))
}

pub(crate) fn pages_for(
    catalog: Option<&Catalog>,
    text_id: u32,
    fallback: &str,
) -> [NpcTextPage; 8] {
    if let Some(catalog) = catalog {
        let pages = catalog.npc_text_pages(text_id);
        if pages
            .iter()
            .any(|page| !page.text.is_empty() || !page.text_female.is_empty())
        {
            return pages;
        }
    }
    let mut pages = std::array::from_fn(|_| NpcTextPage::default());
    if !fallback.is_empty() {
        pages[0] = NpcTextPage {
            probability: 1.0,
            text: fallback.to_string(),
            text_female: fallback.to_string(),
        };
    }
    pages
}
