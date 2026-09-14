use tokio::io::AsyncWriteExt;
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    AllowedClass, AllowedRace, Class, CreatureFamily, Gender, Gold, InventoryType,
    ItemClassAndSubClass, ItemFlag, ItemQuality, Level, MovementBlock, MovementBlock_MovementFlags,
    MovementBlock_UpdateFlag, MovementBlock_UpdateFlag_All, MovementBlock_UpdateFlag_Living,
    Object, ObjectType, Power, Race, SMSG_CREATURE_QUERY_RESPONSE,
    SMSG_CREATURE_QUERY_RESPONSE_found, SMSG_DESTROY_OBJECT, SMSG_GAMEOBJECT_QUERY_RESPONSE,
    SMSG_GAMEOBJECT_QUERY_RESPONSE_found, SMSG_ITEM_QUERY_SINGLE_RESPONSE,
    SMSG_ITEM_QUERY_SINGLE_RESPONSE_found, SMSG_MONSTER_MOVE, SMSG_MONSTER_MOVE_MonsterMoveType,
    SMSG_NAME_QUERY_RESPONSE, SMSG_STANDSTATE_UPDATE, SMSG_UPDATE_OBJECT, ServerMessage,
    SplineFlag, UnitStandState, UpdateGameObject, UpdateMask, UpdatePlayer, UpdateUnit,
};

use crate::appearance::{class, gender, power, race};
use crate::catalog::ItemRow;
use crate::creature::{Creature, is_creature_guid};
use crate::gameobject::GameObject;
use crate::player::Player;
use crate::protocol::geometry::vector3d;

const UNIT_FLAG_NON_ATTACKABLE: i32 = 0x0000_0002;
const UNIT_FLAG_IMMUNE_TO_PC: i32 = 0x0000_0100;
const UNIT_DYNFLAG_LOOTABLE: i32 = 0x01;
const UNIT_DYNFLAG_DEAD: i32 = 0x20;
const STAND_STATE_DEAD: u8 = 7;

fn creature_unit_flags(hostile: bool, dead: bool) -> i32 {
    if dead || !hostile {
        UNIT_FLAG_NON_ATTACKABLE | UNIT_FLAG_IMMUNE_TO_PC
    } else {
        0
    }
}

fn creature_dynamic_flags(dead: bool, lootable: bool) -> i32 {
    if !dead {
        return 0;
    }
    if lootable {
        UNIT_DYNFLAG_DEAD | UNIT_DYNFLAG_LOOTABLE
    } else {
        UNIT_DYNFLAG_DEAD
    }
}

fn player_dynamic_flags(dead: bool) -> i32 {
    if dead { UNIT_DYNFLAG_DEAD } else { 0 }
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
        race: race(player.race),
        gender: gender(player.gender),
        class: class(player.class),
    }
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

pub async fn appear_gameobjects<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    objects: &[GameObject],
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    if objects.is_empty() {
        return Ok(());
    }
    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: objects.iter().map(create_gameobject_object).collect(),
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

pub async fn gameobject_query<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    entry: u32,
    object: Option<&GameObject>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let packet = match object {
        Some(object) => SMSG_GAMEOBJECT_QUERY_RESPONSE {
            entry_id: entry,
            found: Some(SMSG_GAMEOBJECT_QUERY_RESPONSE_found {
                info_type: object.object_type as u32,
                display_id: object.display_id as u32,
                name1: object.name.clone(),
                name2: String::new(),
                name3: String::new(),
                name4: String::new(),
                name5: String::new(),
                raw_data: object.query_data(),
            }),
        },
        None => SMSG_GAMEOBJECT_QUERY_RESPONSE {
            entry_id: entry | 0x8000_0000,
            found: None,
        },
    };
    packet
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn creature_moved<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    from: wow_shared::Position,
    to: wow_shared::Position,
    duration_ms: u32,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_MONSTER_MOVE {
        guid: Guid::new(guid),
        spline_point: vector3d(from),
        spline_id: 0,
        move_type: SMSG_MONSTER_MOVE_MonsterMoveType::Normal,
        spline_flags: SplineFlag::empty(),
        duration: duration_ms,
        splines: vec![vector3d(to)],
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn money<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    guid: u64,
    copper: u32,
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
                    .set_player_field_coinage(copper as i32)
                    .finalize(),
            ),
        }],
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn item_query<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    entry: u32,
    item: Option<&ItemRow>,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let packet = match item {
        Some(item) => {
            let class_and_sub_class = ItemClassAndSubClass::try_from(
                (u64::from(item.subclass) << 32) | u64::from(item.class),
            )
            .unwrap_or(ItemClassAndSubClass::Consumable);
            SMSG_ITEM_QUERY_SINGLE_RESPONSE {
                item: entry,
                found: Some(SMSG_ITEM_QUERY_SINGLE_RESPONSE_found {
                    class_and_sub_class,
                    name1: item.name.clone(),
                    display_id: item.display_id,
                    quality: ItemQuality::try_from(item.quality).unwrap_or(ItemQuality::Normal),
                    flags: ItemFlag::new(item.flags),
                    buy_price: Gold::new(item.buy_price),
                    sell_price: Gold::new(item.sell_price),
                    inventory_type: InventoryType::try_from(item.inventory_type)
                        .unwrap_or(InventoryType::NonEquip),
                    allowed_class: AllowedClass::all(),
                    allowed_race: AllowedRace::all(),
                    item_level: Level::new(item.item_level),
                    required_level: Level::new(item.required_level),
                    stackable: item.stackable,
                    max_durability: item.max_durability,
                    description: item.description.clone(),
                    delay: item.delay,
                    armor: item.armor,
                    ..Default::default()
                }),
            }
        }
        None => SMSG_ITEM_QUERY_SINGLE_RESPONSE {
            item: entry,
            found: None,
        },
    };
    packet
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
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
    lootable: bool,
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
                .set_unit_flags(UNIT_FLAG_NON_ATTACKABLE | UNIT_FLAG_IMMUNE_TO_PC)
                .set_unit_dynamic_flags(creature_dynamic_flags(true, lootable))
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
                .set_unit_dynamic_flags(player_dynamic_flags(true))
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
    let flags = creature_unit_flags(creature.hostile, creature.dead);
    if flags != 0 {
        update = update.set_unit_flags(flags);
    }
    if creature.dead {
        update = update
            .set_unit_dynamic_flags(creature_dynamic_flags(true, creature.lootable))
            .set_unit_bytes_1(STAND_STATE_DEAD, 0, 0, 0);
        Object::CreateObject2 {
            guid3: guid,
            mask2: UpdateMask::Unit(update.finalize()),
            movement2: MovementBlock {
                update_flag: MovementBlock_UpdateFlag::empty()
                    .set_living(MovementBlock_UpdateFlag_Living::HasPosition {
                        orientation: creature.position.orientation,
                        position,
                    })
                    .set_all(MovementBlock_UpdateFlag_All { unknown1: 1 }),
            },
            object_type: ObjectType::Unit,
        }
    } else {
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
}

pub(crate) fn create_gameobject_object(object: &GameObject) -> Object {
    let guid = Guid::new(object.guid);
    let position = vector3d(object.position);
    let anim = if object.anim_progress == 0 {
        100
    } else {
        object.anim_progress
    };
    Object::CreateObject2 {
        guid3: guid,
        mask2: UpdateMask::GameObject(
            UpdateGameObject::builder()
                .set_object_guid(guid)
                .set_object_entry(object.entry as i32)
                .set_object_scale_x(object.scale)
                .set_gameobject_displayid(object.display_id)
                .set_gameobject_flags(object.flags)
                .set_gameobject_state(object.state)
                .set_gameobject_faction(object.faction)
                .set_gameobject_type_id(object.object_type)
                .set_gameobject_animprogress(anim)
                .finalize(),
        ),
        movement2: MovementBlock {
            update_flag: MovementBlock_UpdateFlag::empty()
                .set_living(MovementBlock_UpdateFlag_Living::HasPosition {
                    orientation: object.position.orientation,
                    position,
                })
                .set_all(MovementBlock_UpdateFlag_All { unknown1: 1 }),
        },
        object_type: ObjectType::GameObject,
    }
}

pub(crate) fn create_player_object(player: &Player, as_self: bool) -> Object {
    let guid = Guid::new(player.guid);
    let position = vector3d(player.position);
    let mut update = UpdatePlayer::builder()
        .set_object_guid(guid)
        .set_unit_bytes_0(
            race(player.race),
            class(player.class),
            gender(player.gender),
            power(player.class),
        )
        .set_object_scale_x(1.0)
        .set_unit_health(player.health)
        .set_unit_maxhealth(player.max_health)
        .set_unit_level(1)
        .set_unit_factiontemplate(player.faction)
        .set_unit_displayid(player.display_id)
        .set_unit_nativedisplayid(player.display_id)
        .set_player_features(
            player.appearance.skin,
            player.appearance.face,
            player.appearance.hair_style,
            player.appearance.hair_color,
        )
        .set_player_bytes_2(player.appearance.facial_hair, 0, 0, 0)
        .set_unit_bytes_1(player.stand_state, 0, 0, 0)
        .set_player_field_coinage(player.copper as i32);
    update = super::quest::apply_quest_log(update, player);
    if player.health <= 0 {
        update = update
            .set_unit_dynamic_flags(player_dynamic_flags(true))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_creature_is_lootable_only_with_drops() {
        assert_eq!(
            creature_dynamic_flags(true, true),
            UNIT_DYNFLAG_DEAD | UNIT_DYNFLAG_LOOTABLE
        );
        assert_eq!(creature_dynamic_flags(true, true), 0x21);
        assert_eq!(creature_dynamic_flags(true, false), UNIT_DYNFLAG_DEAD);
        assert_eq!(creature_dynamic_flags(true, false), 0x20);
        assert_eq!(creature_dynamic_flags(false, true), 0);
        assert_eq!(
            creature_unit_flags(true, true),
            UNIT_FLAG_NON_ATTACKABLE | UNIT_FLAG_IMMUNE_TO_PC
        );
        assert_eq!(creature_unit_flags(true, false), 0);
    }

    #[test]
    fn dead_player_is_not_lootable() {
        assert_eq!(player_dynamic_flags(true), UNIT_DYNFLAG_DEAD);
        assert_eq!(player_dynamic_flags(true) & UNIT_DYNFLAG_LOOTABLE, 0);
        assert_eq!(player_dynamic_flags(false), 0);
    }
}
