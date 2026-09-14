use tokio::io::AsyncWriteExt;
use wow_shared::{CharacterMap, CharacterTemplate};
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    Character, CreatureFamily, DateTime, Level, LogoutResult, LogoutSpeed, SMSG_ACCOUNT_DATA_TIMES,
    SMSG_ACTION_BUTTONS, SMSG_BINDPOINTUPDATE, SMSG_CHAR_CREATE, SMSG_CHAR_DELETE, SMSG_CHAR_ENUM,
    SMSG_INITIAL_SPELLS, SMSG_INITIALIZE_FACTIONS, SMSG_LOGIN_SETTIMESPEED,
    SMSG_LOGIN_VERIFY_WORLD, SMSG_LOGOUT_CANCEL_ACK, SMSG_LOGOUT_COMPLETE, SMSG_LOGOUT_RESPONSE,
    SMSG_NEW_WORLD, SMSG_PONG, SMSG_TRANSFER_PENDING, SMSG_TUTORIAL_FLAGS, ServerMessage,
    WorldResult,
};

use crate::appearance::{class, gender, race};
use crate::player::Player;
use crate::protocol::geometry::vector3d;

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
    characters: &[CharacterTemplate],
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_CHAR_ENUM {
        characters: characters.iter().map(enum_character).collect(),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn char_create<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    result: WorldResult,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_CHAR_CREATE { result }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn char_delete<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    result: WorldResult,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_CHAR_DELETE { result }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn logout_response<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    result: LogoutResult,
    speed: LogoutSpeed,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_LOGOUT_RESPONSE { result, speed }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn logout_complete<W>(stream: &mut W, encrypter: &mut EncrypterHalf) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_LOGOUT_COMPLETE {}
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn logout_cancel_ack<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_LOGOUT_CANCEL_ACK {}
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

fn enum_character(character: &CharacterTemplate) -> Character {
    Character {
        guid: Guid::new(character.guid),
        name: character.name.clone(),
        race: race(character.race),
        class: class(character.class),
        gender: gender(character.gender),
        skin: character.appearance.skin,
        face: character.appearance.face,
        hair_style: character.appearance.hair_style,
        hair_color: character.appearance.hair_color,
        facial_hair: character.appearance.facial_hair,
        level: Level::new_player(),
        area: character.area,
        map: character.map_id,
        position: vector3d(character.position),
        guild_id: 0,
        flags: Default::default(),
        first_login: character.first_login,
        pet_display_id: 0,
        pet_level: Level::zero(),
        pet_family: CreatureFamily::None,
        equipment: [Default::default(); 19],
    }
}

pub async fn enter_world<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    character: &CharacterTemplate,
    player: &Player,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let position = vector3d(character.position);
    SMSG_LOGIN_VERIFY_WORLD {
        map: character.map_id,
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
        map: character.map_id,
        area: character.area,
    }
    .tokio_write_encrypted_server(&mut *stream, encrypter)
    .await?;

    super::appear(stream, encrypter, std::slice::from_ref(player), true).await
}

#[allow(dead_code)]
pub async fn transfer_world<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    map_id: CharacterMap,
    position: wow_shared::Position,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let map = map_id;
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
