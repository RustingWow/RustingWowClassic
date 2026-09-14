use tokio::io::AsyncWriteExt;
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    Faction, Gold, Level, Object, QuestGiverStatus, QuestItem, QuestItemRequirement,
    QuestItemReward, QuestObjective, SMSG_QUEST_QUERY_RESPONSE, SMSG_QUESTGIVER_OFFER_REWARD,
    SMSG_QUESTGIVER_QUEST_COMPLETE, SMSG_QUESTGIVER_QUEST_DETAILS, SMSG_QUESTGIVER_QUEST_LIST,
    SMSG_QUESTGIVER_STATUS, SMSG_QUESTLOG_FULL, SMSG_QUESTUPDATE_ADD_KILL,
    SMSG_QUESTUPDATE_COMPLETE, SMSG_UPDATE_OBJECT, ServerMessage, UpdateMask, UpdatePlayer,
    UpdatePlayerBuilder, Vector2d,
};

use crate::catalog::QuestRow;
use crate::player::{Player, QUEST_LOG_SLOTS};
use crate::quest::{self, GossipQuestItem, KillCredit};

pub(crate) fn quest_item(item: GossipQuestItem) -> QuestItem {
    QuestItem {
        quest_id: item.quest_id,
        quest_icon: item.icon,
        level: Level::new(item.level.max(0) as u8),
        title: item.title,
    }
}

fn quest_item_rewards(ids: &[i32], counts: &[i32]) -> Vec<QuestItemReward> {
    ids.iter()
        .zip(counts.iter())
        .filter(|(id, count)| **id > 0 && **count > 0)
        .map(|(id, count)| QuestItemReward {
            item: *id as u32,
            item_count: *count as u32,
        })
        .collect()
}

fn quest_item_requirements(ids: &[i32], counts: &[i32]) -> Vec<QuestItemRequirement> {
    ids.iter()
        .zip(counts.iter())
        .filter(|(id, count)| **id > 0 && **count > 0)
        .map(|(id, count)| QuestItemRequirement {
            item: *id as u32,
            item_count: *count as u32,
            item_display_id: 0,
        })
        .collect()
}

pub async fn questgiver_status<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    status: u32,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTGIVER_STATUS {
        guid: Guid::new(npc),
        status: QuestGiverStatus::try_from(status).unwrap_or(QuestGiverStatus::None),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_list<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    title: String,
    quests: Vec<GossipQuestItem>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTGIVER_QUEST_LIST {
        npc: Guid::new(npc),
        title,
        emote_delay: 0,
        emote: 0,
        quest_items: quests.into_iter().map(quest_item).collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_details<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    quest: QuestRow,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTGIVER_QUEST_DETAILS {
        guid: Guid::new(npc),
        quest_id: quest.entry,
        title: quest.title.clone(),
        details: quest.details.clone(),
        objectives: quest.objectives.clone(),
        // CMaNGOS ActivateAccept: enable the Accept button when opening from gossip.
        auto_finish: true,
        choice_item_rewards: quest_item_rewards(
            &quest.rew_choice_item_id,
            &quest.rew_choice_item_count,
        ),
        item_rewards: quest_item_rewards(&quest.rew_item_id, &quest.rew_item_count),
        money_reward: Gold::new(quest.rew_money.max(0) as u32),
        reward_spell: quest.rew_spell.max(0) as u32,
        emotes: Vec::new(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_query<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    quest: &QuestRow,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let mut rewards = [QuestItemReward::default(); 4];
    for (i, reward) in quest_item_rewards(&quest.rew_item_id, &quest.rew_item_count)
        .into_iter()
        .take(4)
        .enumerate()
    {
        rewards[i] = reward;
    }
    let mut choice_rewards = [QuestItemReward::default(); 6];
    for (i, reward) in quest_item_rewards(&quest.rew_choice_item_id, &quest.rew_choice_item_count)
        .into_iter()
        .take(6)
        .enumerate()
    {
        choice_rewards[i] = reward;
    }
    let mut objectives = [QuestObjective::default(); 4];
    for i in 0..4 {
        let creature_id = if quest.req_creature_or_go_id[i] > 0 {
            quest.req_creature_or_go_id[i] as u32
        } else {
            0
        };
        objectives[i] = QuestObjective {
            creature_id,
            kill_count: quest.req_creature_or_go_count[i].max(0) as u32,
            required_item_id: quest.req_item_id[i].max(0) as u32,
            required_item_count: quest.req_item_count[i].max(0) as u32,
        };
    }
    SMSG_QUEST_QUERY_RESPONSE {
        quest_id: quest.entry,
        quest_method: quest.method.max(0) as u32,
        quest_level: Level::new(quest.quest_level.max(0) as u8),
        zone_or_sort: quest.zone_or_sort as u32,
        quest_type: quest.quest_type.as_protocol(),
        reputation_objective_faction: Faction::None,
        reputation_objective_value: 0,
        required_opposite_faction: Faction::None,
        required_opposite_reputation_value: 0,
        next_quest_in_chain: quest.next_quest_in_chain.max(0) as u32,
        money_reward: Gold::new(quest.rew_money.max(0) as u32),
        max_level_money_reward: Gold::new(quest.rew_money_max_level.max(0) as u32),
        reward_spell: quest.rew_spell.max(0) as u32,
        source_item_id: quest.src_item_id.max(0) as u32,
        quest_flags: quest.quest_flags.max(0) as u32,
        rewards,
        choice_rewards,
        point_map_id: quest.point_map_id.max(0) as u32,
        position: Vector2d {
            x: quest.point_x,
            y: quest.point_y,
        },
        point_opt: 0,
        title: quest.title.clone(),
        objective_text: quest.objectives.clone(),
        details: quest.details.clone(),
        end_text: quest.end_text.clone(),
        objectives,
        objective_texts: quest.objective_texts.clone(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_offer_reward<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    quest: QuestRow,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTGIVER_OFFER_REWARD {
        npc: Guid::new(npc),
        quest_id: quest.entry,
        title: quest.title.clone(),
        offer_reward_text: quest.offer_reward_text.clone(),
        auto_finish: quest.method == 0,
        emotes: Vec::new(),
        choice_item_rewards: quest_item_requirements(
            &quest.rew_choice_item_id,
            &quest.rew_choice_item_count,
        ),
        item_rewards: quest_item_requirements(&quest.rew_item_id, &quest.rew_item_count),
        money_reward: Gold::new(quest.rew_money.max(0) as u32),
        reward_spell: quest.rew_spell.max(0) as u32,
        reward_spell_cast: quest.rew_spell_cast.max(0) as u32,
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_complete<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    quest_id: u32,
    copper: u32,
    items: Vec<(u32, u32)>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTGIVER_QUEST_COMPLETE {
        quest_id,
        unknown: 0x03,
        experience_reward: 0,
        money_reward: Gold::new(copper),
        item_rewards: items
            .into_iter()
            .map(|(item, item_count)| QuestItemReward { item, item_count })
            .collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_log_full<W>(stream: &mut W, encrypter: &mut EncrypterHalf) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTLOG_FULL {}
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn quest_log_update<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    player: &Player,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let packed = Guid::new(player.guid);
    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: vec![Object::Values {
            guid1: packed,
            mask1: UpdateMask::Player(
                apply_quest_log(UpdatePlayer::builder().set_object_guid(packed), player).finalize(),
            ),
        }],
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_kill_credit<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    victim: u64,
    credit: KillCredit,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTUPDATE_ADD_KILL {
        quest_id: credit.quest_id,
        creature_id: credit.creature_id,
        kill_count: credit.kill_count,
        required_kill_count: credit.required,
        guid: Guid::new(victim),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn quest_objectives_done<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    quest_id: u32,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_QUESTUPDATE_COMPLETE { quest_id }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub(crate) fn quest_log_slots(player: &Player) -> Vec<(i32, i32)> {
    (0..QUEST_LOG_SLOTS)
        .map(|slot| {
            player
                .quest_log
                .get(slot)
                .map(|entry| (entry.quest_id as i32, quest::packed_counters(entry)))
                .unwrap_or((0, 0))
        })
        .collect()
}

pub(crate) fn apply_quest_log(
    mut update: UpdatePlayerBuilder,
    player: &Player,
) -> UpdatePlayerBuilder {
    for (slot, (id, packed)) in quest_log_slots(player).into_iter().enumerate() {
        update = set_quest_log_slot(update, slot, id, packed);
    }
    update
}

pub(crate) fn set_quest_log_slot(
    update: UpdatePlayerBuilder,
    slot: usize,
    id: i32,
    packed: i32,
) -> UpdatePlayerBuilder {
    match slot {
        0 => update
            .set_player_quest_log_1_1(id)
            .set_player_quest_log_1_2(packed),
        1 => update
            .set_player_quest_log_2_1(id)
            .set_player_quest_log_2_2(packed),
        2 => update
            .set_player_quest_log_3_1(id)
            .set_player_quest_log_3_2(packed),
        3 => update
            .set_player_quest_log_4_1(id)
            .set_player_quest_log_4_2(packed),
        4 => update
            .set_player_quest_log_5_1(id)
            .set_player_quest_log_5_2(packed),
        5 => update
            .set_player_quest_log_6_1(id)
            .set_player_quest_log_6_2(packed),
        6 => update
            .set_player_quest_log_7_1(id)
            .set_player_quest_log_7_2(packed),
        7 => update
            .set_player_quest_log_8_1(id)
            .set_player_quest_log_8_2(packed),
        8 => update
            .set_player_quest_log_9_1(id)
            .set_player_quest_log_9_2(packed),
        9 => update
            .set_player_quest_log_10_1(id)
            .set_player_quest_log_10_2(packed),
        10 => update
            .set_player_quest_log_11_1(id)
            .set_player_quest_log_11_2(packed),
        11 => update
            .set_player_quest_log_12_1(id)
            .set_player_quest_log_12_2(packed),
        12 => update
            .set_player_quest_log_13_1(id)
            .set_player_quest_log_13_2(packed),
        13 => update
            .set_player_quest_log_14_1(id)
            .set_player_quest_log_14_2(packed),
        14 => update
            .set_player_quest_log_15_1(id)
            .set_player_quest_log_15_2(packed),
        15 => update
            .set_player_quest_log_16_1(id)
            .set_player_quest_log_16_2(packed),
        16 => update
            .set_player_quest_log_17_1(id)
            .set_player_quest_log_17_2(packed),
        17 => update
            .set_player_quest_log_18_1(id)
            .set_player_quest_log_18_2(packed),
        18 => update
            .set_player_quest_log_19_1(id)
            .set_player_quest_log_19_2(packed),
        19 => update
            .set_player_quest_log_20_1(id)
            .set_player_quest_log_20_2(packed),
        _ => update,
    }
}

#[cfg(test)]
mod tests {
    use wow_shared::Position;

    use super::*;
    use crate::player::{Player, QuestLogEntry};

    #[test]
    fn apply_quest_log_writes_quest_ids_into_slots() {
        let mut player = Player::new(1, "Hero", Position::NORTHSHIRE);
        player.quest_log = vec![
            QuestLogEntry {
                quest_id: 33,
                kills: [0; 4],
                complete: false,
            },
            QuestLogEntry {
                quest_id: 783,
                kills: [1, 0, 0, 0],
                complete: false,
            },
        ];
        let slots = quest_log_slots(&player);
        assert_eq!(slots.len(), QUEST_LOG_SLOTS);
        assert_eq!(slots[0].0, 33);
        assert_eq!(slots[1].0, 783);
        assert!(slots[2..].iter().all(|slot| *slot == (0, 0)));
        let _ = apply_quest_log(
            UpdatePlayer::builder().set_object_guid(Guid::new(1)),
            &player,
        )
        .finalize();
    }
}
