use tokio::net::TcpStream;
use wow_shared::{CharacterTemplate, SessionInfo, parse_account};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::{HeaderCrypto, ProofSeed};
use wow_world_messages::Guid;
use wow_world_messages::vanilla::opcodes::ClientOpcodeMessage;
use wow_world_messages::vanilla::{
    Addon, Addon_InfoBlock, Addon_UrlInfo, AddonType, Area, CMSG_AUTH_SESSION, Character, Class,
    CreatureFamily, Gender, Level, Map, Race, SMSG_ADDON_INFO, SMSG_AUTH_CHALLENGE,
    SMSG_AUTH_RESPONSE, SMSG_CHAR_ENUM, SMSG_PONG, ServerMessage, Vector3d,
    tokio_expect_client_message,
};

use crate::enter_world;
use crate::packet::{Incoming, read_incoming};

pub async fn handle_client(
    mut stream: TcpStream,
    auth_internal_url: String,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let seed = ProofSeed::new();
    SMSG_AUTH_CHALLENGE {
        server_seed: seed.seed(),
    }
    .tokio_write_unencrypted_server(&mut stream)
    .await?;

    let auth = tokio_expect_client_message::<CMSG_AUTH_SESSION, _>(&mut stream).await?;
    let account = parse_account(&auth.username)?;
    let session = fetch_session(&auth_internal_url, &account.username).await?;

    let mut encryption = match seed.into_server_header_crypto(
        &NormalizedString::new(&account.username)?,
        session.session_key,
        auth.client_proof,
        auth.client_seed,
    ) {
        Ok(crypto) => crypto,
        Err(error) => {
            tracing::info!(account = %account.username, %error, "world auth proof failed");
            return Ok(());
        }
    };

    SMSG_AUTH_RESPONSE::AuthOk {
        billing_flags: 0,
        billing_rested: 0,
        billing_time: 0,
    }
    .tokio_write_encrypted_server(&mut stream, encryption.encrypter())
    .await?;

    send_addon_info(&mut stream, &mut encryption, &auth).await?;
    tracing::info!(account = %account.username, "world session authenticated");

    let character = CharacterTemplate::for_account(&account);
    run_session(
        &mut stream,
        &mut encryption,
        character,
        log_unhandled_packets,
    )
    .await
}

async fn fetch_session(auth_internal_url: &str, account: &str) -> anyhow::Result<SessionInfo> {
    let url = format!(
        "{}/internal/sessions/{}",
        auth_internal_url.trim_end_matches('/'),
        account
    );
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        anyhow::bail!(
            "auth session lookup failed for {account}: {}",
            response.status()
        );
    }
    Ok(response.json().await?)
}

async fn send_addon_info(
    stream: &mut TcpStream,
    encryption: &mut HeaderCrypto,
    auth: &CMSG_AUTH_SESSION,
) -> anyhow::Result<()> {
    let addons = auth
        .addon_info
        .iter()
        .map(|_| Addon {
            addon_type: AddonType::Blizzard,
            info_block: Addon_InfoBlock::Unavailable,
            url_info: Addon_UrlInfo::Unavailable,
        })
        .collect();

    SMSG_ADDON_INFO { addons }
        .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
        .await?;
    Ok(())
}

async fn run_session(
    stream: &mut TcpStream,
    encryption: &mut HeaderCrypto,
    character: CharacterTemplate,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let mut in_world = false;

    loop {
        let opcode = match read_incoming(&mut *stream, encryption).await {
            Ok(Incoming::Message(opcode)) => opcode,
            Ok(Incoming::Skipped { opcode, name }) => {
                if log_unhandled_packets {
                    tracing::info!(opcode, name, in_world, "unhandled packet");
                }
                continue;
            }
            Err(error) => {
                tracing::debug!(%error, "client disconnected");
                return Ok(());
            }
        };

        match opcode {
            ClientOpcodeMessage::CMSG_PING(ping) => {
                SMSG_PONG {
                    sequence_id: ping.sequence_id,
                }
                .tokio_write_encrypted_server(&mut *stream, encryption.encrypter())
                .await?;
            }
            ClientOpcodeMessage::CMSG_CHAR_ENUM => {
                send_char_enum(&mut *stream, encryption, &character).await?;
            }
            ClientOpcodeMessage::CMSG_PLAYER_LOGIN(login) => {
                if login.guid != Guid::new(character.guid) {
                    tracing::warn!(guid = ?login.guid, "unknown character guid");
                    continue;
                }
                enter_world::send_enter_world(&mut *stream, encryption, &character).await?;
                in_world = true;
                tracing::info!(name = %character.name, "player entered world");
            }
            other => {
                if log_unhandled_packets {
                    tracing::info!(opcode = %other, in_world, "unhandled packet");
                }
            }
        }
    }
}

async fn send_char_enum(
    stream: &mut TcpStream,
    encryption: &mut HeaderCrypto,
    character: &CharacterTemplate,
) -> anyhow::Result<()> {
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
            map: Map::EasternKingdoms,
            position: Vector3d {
                x: character.x,
                y: character.y,
                z: character.z,
            },
            guild_id: 0,
            flags: Default::default(),
            first_login: false,
            pet_display_id: 0,
            pet_level: Level::zero(),
            pet_family: CreatureFamily::None,
            equipment: [Default::default(); 19],
        }],
    }
    .tokio_write_encrypted_server(stream, encryption.encrypter())
    .await?;
    Ok(())
}
