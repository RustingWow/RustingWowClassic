use tokio::net::TcpStream;
use wow_shared::CharacterTemplate;
use wow_srp::vanilla_header::HeaderCrypto;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    Area, Class, DateTime, Gender, Map, MovementBlock, MovementBlock_MovementFlags,
    MovementBlock_UpdateFlag, MovementBlock_UpdateFlag_All, MovementBlock_UpdateFlag_Living,
    Object, ObjectType, Power, Race, SMSG_ACCOUNT_DATA_TIMES, SMSG_ACTION_BUTTONS,
    SMSG_BINDPOINTUPDATE, SMSG_INITIAL_SPELLS, SMSG_INITIALIZE_FACTIONS, SMSG_LOGIN_SETTIMESPEED,
    SMSG_LOGIN_VERIFY_WORLD, SMSG_TUTORIAL_FLAGS, SMSG_UPDATE_OBJECT, ServerMessage, UpdateMask,
    UpdatePlayer, Vector3d,
};

pub async fn send_enter_world(
    stream: &mut TcpStream,
    encryption: &mut HeaderCrypto,
    character: &CharacterTemplate,
) -> anyhow::Result<()> {
    let guid = Guid::new(character.guid);
    let position = Vector3d {
        x: character.x,
        y: character.y,
        z: character.z,
    };
    SMSG_LOGIN_VERIFY_WORLD {
        map: Map::EasternKingdoms,
        position,
        orientation: character.orientation,
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    SMSG_ACCOUNT_DATA_TIMES { data: [0; 32] }
        .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
        .await?;

    SMSG_TUTORIAL_FLAGS {
        tutorial_data: [u32::MAX; 8],
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    SMSG_INITIAL_SPELLS {
        unknown1: 0,
        initial_spells: Vec::new(),
        cooldowns: Vec::new(),
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    SMSG_ACTION_BUTTONS { data: [0; 120] }
        .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
        .await?;

    SMSG_INITIALIZE_FACTIONS {
        factions: Vec::new(),
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    SMSG_LOGIN_SETTIMESPEED {
        datetime: DateTime::try_from(0x1673_320A).unwrap_or_default(),
        timescale: 0.016_666_668,
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    SMSG_BINDPOINTUPDATE {
        position,
        map: Map::EasternKingdoms,
        area: Area::NorthshireValley,
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    let update_mask = UpdatePlayer::builder()
        .set_object_guid(guid)
        .set_unit_bytes_0(Race::Human, Class::Warrior, Gender::Male, Power::Rage)
        .set_object_scale_x(1.0)
        .set_unit_health(100)
        .set_unit_maxhealth(100)
        .set_unit_level(1)
        .set_unit_factiontemplate(1)
        .set_unit_displayid(49)
        .set_unit_nativedisplayid(49)
        .finalize();

    let update_flag = MovementBlock_UpdateFlag::empty()
        .set_living(MovementBlock_UpdateFlag_Living::Living {
            backwards_running_speed: 4.5,
            backwards_swimming_speed: 0.0,
            fall_time: 0.0,
            flags: MovementBlock_MovementFlags::empty(),
            living_orientation: character.orientation,
            living_position: position,
            running_speed: 7.0,
            swimming_speed: 0.0,
            timestamp: 0,
            turn_rate: std::f32::consts::PI,
            walking_speed: 1.0,
        })
        .set_all(MovementBlock_UpdateFlag_All { unknown1: 1 })
        .set_self();

    SMSG_UPDATE_OBJECT {
        has_transport: 0,
        objects: vec![Object::CreateObject2 {
            guid3: guid,
            mask2: UpdateMask::Player(update_mask),
            movement2: MovementBlock { update_flag },
            object_type: ObjectType::Player,
        }],
    }
    .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
    .await?;

    Ok(())
}
