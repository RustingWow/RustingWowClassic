use crate::catalog::{Catalog, QuestRow};
use crate::creature::unhandled_npc_flags;
use crate::player::Player;
use crate::quest::{self, GossipQuestItem, QuestStateChange};

use super::gossip::dialog_npc;
use super::{Inner, PlayerMailbox, World, WorldEvent};

impl World {
    pub fn questgiver_status(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        let catalog = self.catalog.clone();
        let inner = self.inner.lock().expect("world mutex");
        let Some((presence, creature)) = dialog_npc(&inner, from, player, npc) else {
            return;
        };
        let status = catalog
            .as_ref()
            .map(|catalog| quest::dialog_status(catalog, &presence.player, creature.creature.entry))
            .unwrap_or(quest::QuestDialogStatus::None);
        from.send(WorldEvent::QuestGiverStatus {
            npc,
            status: status.as_protocol(),
        });
    }

    pub fn open_questgiver(&self, from: &PlayerMailbox, player: u64, npc: u64) {
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
        let fallback = creature
            .creature
            .gossip
            .as_ref()
            .map(|gossip| gossip.greeting().text.clone())
            .unwrap_or_default();
        let title = catalog
            .as_ref()
            .map(|catalog| quest::greeting_text(catalog, creature.creature.entry, &fallback))
            .unwrap_or(fallback);
        let quests = quest_items_for(
            catalog.as_deref(),
            &presence.player,
            creature.creature.entry,
        );
        from.send(WorldEvent::QuestList { npc, title, quests });
    }

    pub fn query_quest_details(&self, from: &PlayerMailbox, player: u64, npc: u64, quest_id: u32) {
        let catalog = self.catalog.clone();
        let inner = self.inner.lock().expect("world mutex");
        if dialog_npc(&inner, from, player, npc).is_none() {
            tracing::info!(
                player,
                npc,
                quest_id,
                "dropped quest details: npc not in dialog range"
            );
            return;
        }
        let Some(quest) = catalog
            .as_ref()
            .and_then(|catalog| catalog.quest(quest_id).cloned())
        else {
            tracing::info!(
                player,
                npc,
                quest_id,
                "dropped quest details: unknown quest"
            );
            return;
        };
        send_quest_details(from, npc, quest);
    }

    pub fn accept_quest(&self, from: &PlayerMailbox, player: u64, npc: u64, quest_id: u32) {
        let Some(catalog) = self.catalog.clone() else {
            return;
        };
        let mut inner = self.inner.lock().expect("world mutex");
        let Some((_, creature)) = dialog_npc(&inner, from, player, npc) else {
            return;
        };
        let npc_entry = creature.creature.entry;
        if !catalog.creature_quest_starts(npc_entry).contains(&quest_id) {
            return;
        }
        let Some(quest) = catalog.quest(quest_id).cloned() else {
            return;
        };
        let Some(presence) = inner.players.get_mut(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        };
        if presence.player.quest_log.len() >= crate::player::QUEST_LOG_SLOTS {
            from.send(WorldEvent::QuestLogFull);
            return;
        }
        if !quest::accept(&mut presence.player, &quest) {
            return;
        }
        let snapshot = presence.player.clone();
        let change = QuestStateChange::Active {
            quest_id,
            kills: [0; 4],
            complete: quest.auto_complete(),
        };
        from.send(WorldEvent::GossipClosed);
        from.send(WorldEvent::QuestLogUpdate { player: snapshot });
        from.send(WorldEvent::QuestStateChanged {
            guid: player,
            change,
        });
    }

    pub fn complete_quest(&self, from: &PlayerMailbox, player: u64, npc: u64, quest_id: u32) {
        let Some(catalog) = self.catalog.clone() else {
            return;
        };
        let inner = self.inner.lock().expect("world mutex");
        let Some((presence, creature)) = dialog_npc(&inner, from, player, npc) else {
            return;
        };
        let ends = catalog
            .creature_quest_ends(creature.creature.entry)
            .contains(&quest_id);
        let Some(quest) = catalog.quest(quest_id).cloned() else {
            return;
        };
        let Some(entry) = quest::log_entry(&presence.player, quest_id).cloned() else {
            drop(inner);
            tracing::info!(
                player,
                npc,
                quest_id,
                "questgiver complete without quest log; showing details"
            );
            self.query_quest_details(from, player, npc, quest_id);
            return;
        };
        if !ends || !quest::is_complete(&quest, &entry) {
            return;
        }
        from.send(WorldEvent::QuestOfferReward { npc, quest });
    }

    pub fn choose_quest_reward(
        &self,
        from: &PlayerMailbox,
        player: u64,
        npc: u64,
        quest_id: u32,
        reward: u32,
    ) {
        let Some(catalog) = self.catalog.clone() else {
            return;
        };
        let mut inner = self.inner.lock().expect("world mutex");
        let Some((_, creature)) = dialog_npc(&inner, from, player, npc) else {
            return;
        };
        if !catalog
            .creature_quest_ends(creature.creature.entry)
            .contains(&quest_id)
        {
            return;
        }
        let Some(quest) = catalog.quest(quest_id).cloned() else {
            return;
        };
        let Some(presence) = inner.players.get_mut(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        };
        let Some(turned) = quest::turn_in(&mut presence.player, &quest, reward) else {
            return;
        };
        for (item_id, count) in &turned.items {
            let stackable = catalog
                .item(*item_id)
                .map(|item| item.stackable)
                .unwrap_or(1);
            let _ = presence.player.add_item(*item_id, *count, stackable);
        }
        if turned.copper > 0 {
            presence.player.copper = presence.player.copper.saturating_add(turned.copper);
        }
        let snapshot = presence.player.clone();
        let copper = snapshot.copper;
        from.send(WorldEvent::GossipClosed);
        from.send(WorldEvent::QuestTurnedIn {
            quest_id: turned.quest_id,
            copper: turned.copper,
            items: turned.items,
        });
        from.send(WorldEvent::MoneyChanged {
            guid: player,
            copper,
        });
        from.send(WorldEvent::QuestLogUpdate { player: snapshot });
        from.send(WorldEvent::QuestStateChanged {
            guid: player,
            change: QuestStateChange::Rewarded { quest_id },
        });
    }

    pub fn abandon_quest(&self, from: &PlayerMailbox, player: u64, slot: u8) {
        let mut inner = self.inner.lock().expect("world mutex");
        let Some(presence) = inner.players.get_mut(&player) else {
            return;
        };
        if !presence.mailbox.same_channel(from) {
            return;
        };
        let Some(quest_id) = quest::abandon_slot(&mut presence.player, slot as usize) else {
            return;
        };
        let snapshot = presence.player.clone();
        from.send(WorldEvent::QuestLogUpdate { player: snapshot });
        from.send(WorldEvent::QuestStateChanged {
            guid: player,
            change: QuestStateChange::Removed { quest_id },
        });
    }
}

pub(crate) fn quest_items_for(
    catalog: Option<&Catalog>,
    player: &Player,
    npc_entry: u32,
) -> Vec<GossipQuestItem> {
    catalog
        .map(|catalog| quest::gossip_quest_items(catalog, player, npc_entry))
        .unwrap_or_default()
}

pub(crate) fn send_quest_details(from: &PlayerMailbox, npc: u64, quest: QuestRow) {
    quest::log_unhandled_fields(&quest);
    tracing::info!(
        npc,
        quest_id = quest.entry,
        title = quest.title.as_str(),
        "sending quest details"
    );
    from.send(WorldEvent::QuestDetails { npc, quest });
}

pub(crate) fn credit_quest_kills(
    inner: &mut Inner,
    catalog: &Catalog,
    attacker: u64,
    victim: u64,
    events: &mut Vec<WorldEvent>,
) {
    let Some(entry) = inner
        .creatures
        .get(&victim)
        .map(|state| state.creature.entry)
    else {
        return;
    };
    let Some(presence) = inner.players.get_mut(&attacker) else {
        return;
    };
    let credits = quest::credit_kill(&mut presence.player, catalog, entry);
    if credits.is_empty() {
        return;
    }
    let snapshot = presence.player.clone();
    for credit in credits {
        let complete = credit.complete;
        let quest_id = credit.quest_id;
        let kills = presence
            .player
            .quest_log
            .iter()
            .find(|entry| entry.quest_id == quest_id)
            .map(|entry| entry.kills)
            .unwrap_or([0; 4]);
        events.push(WorldEvent::QuestKillCredit {
            victim,
            credit: credit.clone(),
        });
        if complete {
            events.push(WorldEvent::QuestObjectivesDone { quest_id });
        }
        events.push(WorldEvent::QuestStateChanged {
            guid: attacker,
            change: QuestStateChange::Active {
                quest_id,
                kills,
                complete,
            },
        });
    }
    events.push(WorldEvent::QuestLogUpdate { player: snapshot });
}
