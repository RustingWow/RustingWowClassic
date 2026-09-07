use tokio::io::AsyncWriteExt;
use wow_shared::{CharacterTemplate, MAP_KALIMDOR};
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    Area, Character, Class, CreatureFamily, DamageInfo, DateTime, Gender, GossipItem, HitInfo,
    Language, Level, Map, MovementBlock, MovementBlock_MovementFlags, MovementBlock_UpdateFlag,
    MovementBlock_UpdateFlag_All, MovementBlock_UpdateFlag_Living, NpcTextUpdate,
    NpcTextUpdateEmote, Object, ObjectType, PlayerChatTag, Power, Race, SMSG_ACCOUNT_DATA_TIMES,
    SMSG_ACTION_BUTTONS, SMSG_ATTACKERSTATEUPDATE, SMSG_ATTACKSTART, SMSG_ATTACKSTOP,
    SMSG_BINDPOINTUPDATE, SMSG_CHAR_ENUM, SMSG_CHAT_PLAYER_NOT_FOUND, SMSG_CREATURE_QUERY_RESPONSE,
    SMSG_CREATURE_QUERY_RESPONSE_found, SMSG_DESTROY_OBJECT, SMSG_GOSSIP_COMPLETE,
    SMSG_GOSSIP_MESSAGE, SMSG_INITIAL_SPELLS, SMSG_INITIALIZE_FACTIONS, SMSG_LOGIN_SETTIMESPEED,
    SMSG_LOGIN_VERIFY_WORLD, SMSG_MESSAGECHAT, SMSG_MESSAGECHAT_ChatType, SMSG_NAME_QUERY_RESPONSE,
    SMSG_NEW_WORLD, SMSG_NPC_TEXT_UPDATE, SMSG_PONG, SMSG_STANDSTATE_UPDATE, SMSG_TRANSFER_PENDING,
    SMSG_TUTORIAL_FLAGS, SMSG_UPDATE_OBJECT, ServerMessage, UnitStandState, UpdateMask,
    UpdatePlayer, UpdateUnit,
};

use crate::creature::{Creature, GossipMenu, is_creature_guid};
use crate::player::Player;
use crate::protocol::geometry::vector3d;
use crate::world::{Attack, ChatDelivery, MeleeHit, SpokenChat};

const UNIT_FLAG_NON_ATTACKABLE: i32 = 0x0000_0002;
const UNIT_FLAG_IMMUNE_TO_PC: i32 = 0x0000_0100;
const UNIT_DYNFLAG_DEAD: i32 = 0x20;
const STAND_STATE_DEAD: u8 = 7;

const HUMAN_MALE_DISPLAY_ID: i32 = 49;

pub async fn pong<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    sequence_id: u32,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_PONG { sequence_id }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn character_list<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    character: &CharacterTemplate,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_CHAR_ENUM {
        characters: vec![Character {
            guid: Guid::new(character.guid),
            name: character.name.clone(),
            race: Race::Human,
            class: Class::Warrior,
            gender: Gender::Male,
            skin: 0,
            face: 0,
            hair_style: 0,
            hair_color: 0,
            facial_hair: 0,
            level: Level::new_player(),
            area: Area::NorthshireValley,
            map: vanilla_map(character.map_id),
            position: vector3d(character.position),
            guild_id: 0,
            flags: Default::default(),
            first_login: false,
            pet_display_id: 0,
            pet_level: Level::zero(),
            pet_family: CreatureFamily::None,
            equipment: [Default::default(); 19],
        }],
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn enter_world<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    character: &CharacterTemplate,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let position = vector3d(character.position);
    SMSG_LOGIN_VERIFY_WORLD {
        map: vanilla_map(character.map_id),
        position,
        orientation: character.position.orientation,
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    SMSG_ACCOUNT_DATA_TIMES { data: [0; 32] }
        .tokio_write_encrypted_server(&mut *stream, encrypter)
        .await?;

    SMSG_TUTORIAL_FLAGS {
        tutorial_data: [u32::MAX; 8],
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    SMSG_INITIAL_SPELLS {
        unknown1: 0,
        initial_spells: Vec::new(),
        cooldowns: Vec::new(),
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    SMSG_ACTION_BUTTONS { data: [0; 120] }
        .tokio_write_encrypted_server(&mut *stream, encrypter)
        .await?;

    SMSG_INITIALIZE_FACTIONS {
        factions: Vec::new(),
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    SMSG_LOGIN_SETTIMESPEED {
        datetime: DateTime::try_from(0x1673_320A).unwrap_or_default(),
        timescale: 0.016_666_668,
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    SMSG_BINDPOINTUPDATE {
        position,
        map: vanilla_map(character.map_id),
        area: Area::NorthshireValley,
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    appear(stream, encrypter, &[Player::from(character)], true).await
}

pub async fn appear<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    players: &[Player],
    as_self: bool,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    if players.is_empty() {
        return Ok(());
    }
    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: players
            .iter()
            .map(|player| create_player_object(player, as_self))
            .collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn hide_player<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_DESTROY_OBJECT {
        guid: Guid::new(guid),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn name_reply<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    player: &Player,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_NAME_QUERY_RESPONSE {
        guid: Guid::new(guid),
        character_name: player.name.clone(),
        realm_name: String::new(),
        race: Race::Human,
        gender: Gender::Male,
        class: Class::Warrior,
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn chat_message<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    spoken: &SpokenChat,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let guid = Guid::new(spoken.from_guid);
    let chat_type = match spoken.delivery {
        ChatDelivery::Say => SMSG_MESSAGECHAT_ChatType::Say {
            chat_credit: guid,
            speech_bubble_credit: guid,
        },
        ChatDelivery::Yell => SMSG_MESSAGECHAT_ChatType::Yell {
            chat_credit: guid,
            speech_bubble_credit: guid,
        },
        ChatDelivery::Emote => SMSG_MESSAGECHAT_ChatType::Emote { sender2: guid },
        ChatDelivery::Whisper => SMSG_MESSAGECHAT_ChatType::Whisper { sender2: guid },
        ChatDelivery::WhisperInform => SMSG_MESSAGECHAT_ChatType::WhisperInform { sender2: guid },
    };
    SMSG_MESSAGECHAT {
        chat_type,
        language: Language::Universal,
        message: spoken.text.clone(),
        tag: PlayerChatTag::None,
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn chat_player_not_found<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    name: String,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_CHAT_PLAYER_NOT_FOUND { name }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn appear_creatures<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    creatures: &[Creature],
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    if creatures.is_empty() {
        return Ok(());
    }
    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: creatures.iter().map(create_creature_object).collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn creature_query<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    entry: u32,
    creature: Option<&Creature>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let packet = match creature {
        Some(creature) => SMSG_CREATURE_QUERY_RESPONSE {
            creature_entry: entry,
            found: Some(SMSG_CREATURE_QUERY_RESPONSE_found {
                name1: creature.name.clone(),
                name2: String::new(),
                name3: String::new(),
                name4: String::new(),
                sub_name: creature.sub_name.clone(),
                type_flags: 0,
                creature_type: creature.creature_type,
                creature_family: creature.family,
                creature_rank: 0,
                unknown0: 0,
                spell_data_id: 0,
                display_id: creature.display_id as u32,
                civilian: u8::from(creature.civilian),
                racial_leader: 0,
            }),
        },
        None => SMSG_CREATURE_QUERY_RESPONSE {
            creature_entry: entry | 0x8000_0000,
            found: None,
        },
    };
    packet
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn gossip_opened<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    npc: u64,
    menu: GossipMenu,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    npc_text_update(&mut *stream, encrypter, menu.text_id, &menu.text).await?;
    SMSG_GOSSIP_MESSAGE {
        guid: Guid::new(npc),
        title_text_id: menu.text_id,
        gossips: menu
            .options
            .into_iter()
            .map(|option| GossipItem {
                id: option.id,
                item_icon: 0,
                coded: false,
                message: option.text,
            })
            .collect(),
        quests: Vec::new(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn gossip_closed<W>(stream: &mut W, encrypter: &mut EncrypterHalf) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_GOSSIP_COMPLETE {}
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn npc_text_update<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    text_id: u32,
    text: &str,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_NPC_TEXT_UPDATE {
        text_id,
        texts: npc_text_pages(text),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

fn npc_text_pages(text: &str) -> [NpcTextUpdate; 8] {
    let mut pages = std::array::from_fn(|_| NpcTextUpdate {
        probability: 0.0,
        texts: [String::new(), String::new()],
        language: Language::Universal,
        emotes: [NpcTextUpdateEmote::default(); 3],
    });
    pages[0] = NpcTextUpdate {
        probability: 1.0,
        texts: [text.to_string(), text.to_string()],
        language: Language::Universal,
        emotes: [NpcTextUpdateEmote::default(); 3],
    };
    pages
}

pub async fn attack_start<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    attack: Attack,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_ATTACKSTART {
        attacker: Guid::new(attack.attacker),
        victim: Guid::new(attack.victim),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn attack_stop<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    attack: Attack,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_ATTACKSTOP {
        player: Guid::new(attack.attacker),
        enemy: Guid::new(attack.victim),
        unknown1: 0,
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn melee_hit<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    hit: MeleeHit,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_ATTACKERSTATEUPDATE {
        hit_info: HitInfo::AffectsVictim,
        attacker: Guid::new(hit.attacker),
        target: Guid::new(hit.victim),
        total_damage: hit.damage,
        damages: vec![DamageInfo {
            spell_school_mask: 1,
            damage_float: hit.damage as f32,
            damage_uint: hit.damage,
            absorb: 0,
            resist: 0,
        }],
        damage_state: 0,
        unknown1: 0,
        spell_id: 0,
        blocked_amount: 0,
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;
    unit_health(
        stream,
        encrypter,
        hit.victim,
        hit.victim_health,
        hit.victim_max_health,
        hit.victim_dead,
    )
    .await
}

pub async fn stand_state_ack<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    state: u8,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_STANDSTATE_UPDATE {
        state: UnitStandState::try_from(state).unwrap_or(UnitStandState::Stand),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn player_stand_state<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    state: u8,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let packed = Guid::new(guid);
    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: vec![Object::Values {
            guid1: packed,
            mask1: UpdateMask::Player(
                UpdatePlayer::builder()
                    .set_object_guid(packed)
                    .set_unit_bytes_1(state, 0, 0, 0)
                    .finalize(),
            ),
        }],
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn unit_health<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    health: i32,
    max_health: i32,
    dead: bool,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let packed = Guid::new(guid);
    let mask = if is_creature_guid(guid) {
        let mut update = UpdateUnit::builder()
            .set_object_guid(packed)
            .set_unit_health(health)
            .set_unit_maxhealth(max_health);
        if dead {
            update = update
                .set_unit_dynamic_flags(UNIT_DYNFLAG_DEAD)
                .set_unit_bytes_1(STAND_STATE_DEAD, 0, 0, 0);
        }
        UpdateMask::Unit(update.finalize())
    } else {
        let mut update = UpdatePlayer::builder()
            .set_object_guid(packed)
            .set_unit_health(health)
            .set_unit_maxhealth(max_health);
        if dead {
            update = update
                .set_unit_dynamic_flags(UNIT_DYNFLAG_DEAD)
                .set_unit_bytes_1(STAND_STATE_DEAD, 0, 0, 0);
        }
        UpdateMask::Player(update.finalize())
    };
    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: vec![Object::Values {
            guid1: packed,
            mask1: mask,
        }],
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

fn create_creature_object(creature: &Creature) -> Object {
    let guid = Guid::new(creature.guid);
    let position = vector3d(creature.position);
    let gender = if creature.family == CreatureFamily::Wolf {
        Gender::None
    } else {
        Gender::Male
    };
    let mut update = UpdateUnit::builder()
        .set_object_guid(guid)
        .set_object_entry(creature.entry as i32)
        .set_object_scale_x(1.0)
        .set_unit_bytes_0(Race::Human, Class::Warrior, gender, Power::Mana)
        .set_unit_health(creature.health)
        .set_unit_maxhealth(creature.max_health)
        .set_unit_level(creature.level)
        .set_unit_factiontemplate(creature.faction)
        .set_unit_displayid(creature.display_id)
        .set_unit_nativedisplayid(creature.display_id)
        .set_unit_npc_flags(creature.npc_flags)
        .set_unit_combatreach(1.5)
        .set_unit_boundingradius(0.306)
        .set_unit_baseattacktime(2000);
    if !creature.hostile {
        update = update.set_unit_flags(UNIT_FLAG_NON_ATTACKABLE | UNIT_FLAG_IMMUNE_TO_PC);
    }
    if creature.dead {
        update = update
            .set_unit_dynamic_flags(UNIT_DYNFLAG_DEAD)
            .set_unit_bytes_1(STAND_STATE_DEAD, 0, 0, 0);
    }
    Object::CreateObject2 {
        guid3: guid,
        mask2: UpdateMask::Unit(update.finalize()),
        movement2: MovementBlock {
            update_flag: MovementBlock_UpdateFlag::empty()
                .set_living(MovementBlock_UpdateFlag_Living::Living {
                    backwards_running_speed: 4.5,
                    backwards_swimming_speed: 0.0,
                    fall_time: 0.0,
                    flags: MovementBlock_MovementFlags::empty(),
                    living_orientation: creature.position.orientation,
                    living_position: position,
                    running_speed: 7.0,
                    swimming_speed: 0.0,
                    timestamp: 0,
                    turn_rate: std::f32::consts::PI,
                    walking_speed: 2.5,
                })
                .set_all(MovementBlock_UpdateFlag_All { unknown1: 1 }),
        },
        object_type: ObjectType::Unit,
    }
}

fn create_player_object(player: &Player, as_self: bool) -> Object {
    let guid = Guid::new(player.guid);
    let position = vector3d(player.position);
    let mut update = UpdatePlayer::builder()
        .set_object_guid(guid)
        .set_unit_bytes_0(Race::Human, Class::Warrior, Gender::Male, Power::Rage)
        .set_object_scale_x(1.0)
        .set_unit_health(player.health)
        .set_unit_maxhealth(player.max_health)
        .set_unit_level(1)
        .set_unit_factiontemplate(1)
        .set_unit_displayid(HUMAN_MALE_DISPLAY_ID)
        .set_unit_nativedisplayid(HUMAN_MALE_DISPLAY_ID)
        .set_unit_bytes_1(player.stand_state, 0, 0, 0);
    if player.health <= 0 {
        update = update
            .set_unit_dynamic_flags(UNIT_DYNFLAG_DEAD)
            .set_unit_bytes_1(STAND_STATE_DEAD, 0, 0, 0);
    }
    let update_mask = update.finalize();

    let mut update_flag = MovementBlock_UpdateFlag::empty()
        .set_living(MovementBlock_UpdateFlag_Living::Living {
            backwards_running_speed: 4.5,
            backwards_swimming_speed: 0.0,
            fall_time: 0.0,
            flags: MovementBlock_MovementFlags::empty(),
            living_orientation: player.position.orientation,
            living_position: position,
            running_speed: 7.0,
            swimming_speed: 0.0,
            timestamp: 0,
            turn_rate: std::f32::consts::PI,
            walking_speed: 1.0,
        })
        .set_all(MovementBlock_UpdateFlag_All { unknown1: 1 });
    if as_self {
        update_flag = update_flag.set_self();
    }

    Object::CreateObject2 {
        guid3: guid,
        mask2: UpdateMask::Player(update_mask),
        movement2: MovementBlock { update_flag },
        object_type: ObjectType::Player,
    }
}

#[allow(dead_code)]
pub async fn transfer_world<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    map_id: u32,
    position: wow_shared::Position,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let map = vanilla_map(map_id);
    SMSG_TRANSFER_PENDING {
        map,
        has_transport: Default::default(),
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;
    SMSG_NEW_WORLD {
        map,
        position: vector3d(position),
        orientation: position.orientation,
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

fn vanilla_map(map_id: u32) -> Map {
    match map_id {
        MAP_KALIMDOR => Map::Kalimdor,
        _ => Map::EasternKingdoms,
    }
}
