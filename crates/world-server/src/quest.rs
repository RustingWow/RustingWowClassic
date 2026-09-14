use crate::catalog::{Catalog, QuestRow, QuestgiverKind};
use crate::player::{Player, QUEST_LOG_SLOTS, QuestLogEntry};

/// `DIALOG_STATUS_AVAILABLE` in the gossip/quest-list payload.
///
/// Icon `0` (`DIALOG_STATUS_NONE`) is not a clickable available quest: the 1.12.1 client
/// treats it as a one-click complete and sends `CMSG_QUESTGIVER_COMPLETE_QUEST` instead of
/// `CMSG_QUESTGIVER_QUERY_QUEST`.
pub const QUEST_ICON_AVAILABLE: u32 = 5;
/// `DIALOG_STATUS_REWARD_REP` — yellow `?` in the gossip quest list (not overhead status 7).
pub const QUEST_ICON_COMPLETE: u32 = 4;

pub fn has_unhandled_fields(quest: &QuestRow) -> bool {
    quest.quest_flags != 0
        || quest.special_flags != 0
        || quest.prev_quest_id != 0
        || quest.exclusive_group != 0
        || quest.src_item_id > 0
        || quest.req_item_id.iter().any(|id| *id > 0)
}

pub fn log_unhandled_fields(quest: &QuestRow) {
    if !has_unhandled_fields(quest) {
        return;
    }
    tracing::info!(
        quest_id = quest.entry,
        title = quest.title.as_str(),
        quest_flags = quest.quest_flags,
        special_flags = quest.special_flags,
        prev_quest_id = quest.prev_quest_id,
        exclusive_group = quest.exclusive_group,
        src_item_id = quest.src_item_id,
        req_item_id = ?quest.req_item_id,
        "unhandled quest fields"
    );
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GossipQuestItem {
    pub quest_id: u32,
    pub icon: u32,
    pub level: i16,
    pub title: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestDialogStatus {
    None,
    Incomplete,
    Available,
    Reward,
}

impl QuestDialogStatus {
    pub fn as_protocol(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Incomplete => 3,
            Self::Available => 5,
            Self::Reward => 7,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum QuestStateChange {
    Active {
        quest_id: u32,
        kills: [u32; 4],
        complete: bool,
    },
    Rewarded {
        quest_id: u32,
    },
    Removed {
        quest_id: u32,
    },
}

pub fn player_has_quest(player: &Player, quest_id: u32) -> bool {
    player
        .quest_log
        .iter()
        .any(|entry| entry.quest_id == quest_id)
}

pub fn player_rewarded(player: &Player, quest_id: u32) -> bool {
    player.rewarded_quests.contains(&quest_id)
}

pub fn log_entry(player: &Player, quest_id: u32) -> Option<&QuestLogEntry> {
    player
        .quest_log
        .iter()
        .find(|entry| entry.quest_id == quest_id)
}

#[allow(dead_code)]
pub fn log_entry_mut(player: &mut Player, quest_id: u32) -> Option<&mut QuestLogEntry> {
    player
        .quest_log
        .iter_mut()
        .find(|entry| entry.quest_id == quest_id)
}

pub fn is_complete(quest: &QuestRow, entry: &QuestLogEntry) -> bool {
    if entry.complete {
        return true;
    }
    quest.auto_complete()
        || quest
            .kill_objectives()
            .enumerate()
            .all(|(i, (_, count))| entry.kills[i] >= count)
}

pub fn offered_quests<'a>(
    catalog: &'a Catalog,
    player: &Player,
    npc_entry: u32,
) -> Vec<(&'a QuestRow, bool)> {
    let mut listed = Vec::new();
    for quest_id in catalog.creature_quest_starts(npc_entry) {
        let Some(quest) = catalog.quest(*quest_id) else {
            continue;
        };
        if player_rewarded(player, *quest_id) || player_has_quest(player, *quest_id) {
            continue;
        }
        if !quest.offered_to(player.race, player.class, 1) {
            continue;
        }
        listed.push((quest, false));
    }
    for quest_id in catalog.creature_quest_ends(npc_entry) {
        let Some(quest) = catalog.quest(*quest_id) else {
            continue;
        };
        let Some(entry) = log_entry(player, *quest_id) else {
            continue;
        };
        if is_complete(quest, entry) {
            listed.push((quest, true));
        }
    }
    listed
}

pub fn gossip_quest_items(
    catalog: &Catalog,
    player: &Player,
    npc_entry: u32,
) -> Vec<GossipQuestItem> {
    offered_quests(catalog, player, npc_entry)
        .into_iter()
        .map(|(quest, complete)| GossipQuestItem {
            quest_id: quest.entry,
            icon: if complete {
                QUEST_ICON_COMPLETE
            } else {
                QUEST_ICON_AVAILABLE
            },
            level: quest.quest_level,
            title: quest.title.clone(),
        })
        .collect()
}

pub fn dialog_status(catalog: &Catalog, player: &Player, npc_entry: u32) -> QuestDialogStatus {
    let mut available = false;
    let mut incomplete = false;
    for quest_id in catalog.creature_quest_starts(npc_entry) {
        let Some(quest) = catalog.quest(*quest_id) else {
            continue;
        };
        if player_rewarded(player, *quest_id) {
            continue;
        }
        if player_has_quest(player, *quest_id) {
            continue;
        }
        if quest.offered_to(player.race, player.class, 1) {
            available = true;
        }
    }
    for quest_id in catalog.creature_quest_ends(npc_entry) {
        let Some(quest) = catalog.quest(*quest_id) else {
            continue;
        };
        let Some(entry) = log_entry(player, *quest_id) else {
            continue;
        };
        if is_complete(quest, entry) {
            return QuestDialogStatus::Reward;
        }
        incomplete = true;
        let _ = quest;
    }
    if available {
        QuestDialogStatus::Available
    } else if incomplete {
        QuestDialogStatus::Incomplete
    } else {
        QuestDialogStatus::None
    }
}

pub fn greeting_text(catalog: &Catalog, npc_entry: u32, fallback: &str) -> String {
    catalog
        .questgiver_greeting(QuestgiverKind::Creature, npc_entry)
        .unwrap_or(fallback)
        .to_string()
}

pub fn accept(player: &mut Player, quest: &QuestRow) -> bool {
    if player_has_quest(player, quest.entry) || player_rewarded(player, quest.entry) {
        return false;
    }
    if player.quest_log.len() >= QUEST_LOG_SLOTS {
        return false;
    }
    if !quest.offered_to(player.race, player.class, 1) {
        return false;
    }
    player.quest_log.push(QuestLogEntry {
        quest_id: quest.entry,
        kills: [0; 4],
        complete: quest.auto_complete(),
    });
    true
}

pub fn abandon_slot(player: &mut Player, slot: usize) -> Option<u32> {
    if slot >= player.quest_log.len() {
        return None;
    }
    Some(player.quest_log.remove(slot).quest_id)
}

pub fn turn_in(player: &mut Player, quest: &QuestRow, choice: u32) -> Option<TurnInReward> {
    let index = player
        .quest_log
        .iter()
        .position(|entry| entry.quest_id == quest.entry)?;
    if !is_complete(quest, &player.quest_log[index]) {
        return None;
    }
    player.quest_log.remove(index);
    if !player.rewarded_quests.contains(&quest.entry) {
        player.rewarded_quests.push(quest.entry);
    }
    let mut items = Vec::new();
    for (id, count) in quest.rew_item_id.iter().zip(quest.rew_item_count) {
        if *id > 0 && count > 0 {
            items.push((*id as u32, count as u32));
        }
    }
    if choice > 0 {
        let idx = (choice - 1) as usize;
        if let (Some(&id), Some(&count)) = (
            quest.rew_choice_item_id.get(idx),
            quest.rew_choice_item_count.get(idx),
        ) && id > 0
            && count > 0
        {
            items.push((id as u32, count as u32));
        }
    }
    Some(TurnInReward {
        quest_id: quest.entry,
        copper: quest.rew_money.max(0) as u32,
        items,
    })
}

pub fn credit_kill(player: &mut Player, catalog: &Catalog, creature_entry: u32) -> Vec<KillCredit> {
    let mut credits = Vec::new();
    for entry in &mut player.quest_log {
        let Some(quest) = catalog.quest(entry.quest_id) else {
            continue;
        };
        for (i, (target, count)) in quest.kill_objectives().enumerate() {
            if target != creature_entry || entry.kills[i] >= count {
                continue;
            }
            entry.kills[i] += 1;
            let complete = is_complete(quest, entry);
            entry.complete = complete;
            credits.push(KillCredit {
                quest_id: quest.entry,
                creature_id: target,
                kill_count: entry.kills[i],
                required: count,
                complete,
            });
        }
    }
    credits
}

#[derive(Clone, Debug)]
pub struct TurnInReward {
    pub quest_id: u32,
    pub copper: u32,
    pub items: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KillCredit {
    pub quest_id: u32,
    pub creature_id: u32,
    pub kill_count: u32,
    pub required: u32,
    pub complete: bool,
}

pub fn packed_counters(entry: &QuestLogEntry) -> i32 {
    let mut packed = 0i32;
    for (i, count) in entry.kills.iter().enumerate() {
        packed |= ((*count as i32) & 0x3F) << (6 * i);
    }
    packed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::QuestRow;
    use wow_shared::QuestType;

    fn blank_quest() -> QuestRow {
        QuestRow {
            entry: 1,
            title: String::new(),
            details: String::new(),
            objectives: String::new(),
            offer_reward_text: String::new(),
            request_items_text: String::new(),
            end_text: String::new(),
            objective_texts: [String::new(), String::new(), String::new(), String::new()],
            min_level: 0,
            quest_level: 1,
            zone_or_sort: 0,
            quest_type: QuestType::None,
            required_races: Vec::new(),
            required_classes: Vec::new(),
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            next_quest_in_chain: 0,
            method: 2,
            quest_flags: 0,
            special_flags: 0,
            src_item_id: 0,
            req_item_id: [0; 4],
            req_item_count: [0; 4],
            req_creature_or_go_id: [0; 4],
            req_creature_or_go_count: [0; 4],
            rew_money: 0,
            rew_money_max_level: 0,
            rew_item_id: [0; 4],
            rew_item_count: [0; 4],
            rew_choice_item_id: [0; 6],
            rew_choice_item_count: [0; 6],
            rew_spell: 0,
            rew_spell_cast: 0,
            point_map_id: 0,
            point_x: 0.0,
            point_y: 0.0,
        }
    }

    #[test]
    fn unhandled_fields_detect_flags_and_item_reqs() {
        let mut quest = blank_quest();
        assert!(!has_unhandled_fields(&quest));
        quest.quest_flags = 8;
        assert!(has_unhandled_fields(&quest));
        quest.quest_flags = 0;
        quest.req_item_id[0] = 117;
        assert!(has_unhandled_fields(&quest));
    }
}
