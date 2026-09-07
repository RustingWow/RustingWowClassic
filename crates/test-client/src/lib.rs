use std::net::Ipv4Addr;

use tokio::net::TcpStream;
use wow_login_messages::Message;
use wow_login_messages::all::{
    CMD_AUTH_LOGON_CHALLENGE_Client, Locale, Os, Platform, ProtocolVersion, Version,
};
use wow_login_messages::helper::tokio_expect_server_message as tokio_expect_login_server_message;
use wow_login_messages::version_3::{
    CMD_AUTH_LOGON_CHALLENGE_Server, CMD_AUTH_LOGON_PROOF_Client,
    CMD_AUTH_LOGON_PROOF_Client_SecurityFlag, CMD_AUTH_LOGON_PROOF_Server, CMD_REALM_LIST_Client,
    CMD_REALM_LIST_Server,
};
use wow_shared::{SESSION_KEY_LEN, parse_account};
use wow_srp::PublicKey;
use wow_srp::client::SrpClientChallenge;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::ProofSeed;
use wow_world_messages::vanilla::opcodes::ServerOpcodeMessage;
use wow_world_messages::vanilla::{
    AddonInfo, CMSG_AUTH_SESSION, CMSG_CHAR_ENUM, CMSG_PLAYER_LOGIN, ClientMessage,
    SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, tokio_expect_server_message,
    tokio_expect_server_message_encryption,
};

#[derive(Debug)]
pub struct EnterWorldResult {
    pub account: String,
    pub character_name: String,
    pub realm_address: String,
}

pub async fn enter_world(
    auth_addr: &str,
    username: &str,
    password: &str,
) -> anyhow::Result<EnterWorldResult> {
    let account = parse_account(username)?;
    account.verify_password(password)?;

    let mut auth = TcpStream::connect(auth_addr).await?;
    let (session_key, realms) =
        authenticate(&mut auth, &account.username, &account.password).await?;
    let realm = realms
        .realms
        .first()
        .ok_or_else(|| anyhow::anyhow!("auth server returned an empty realm list"))?;

    let mut world = TcpStream::connect(&realm.address).await?;
    let character_name =
        world_login(&mut world, &account.username, session_key, realm.realm_id).await?;

    Ok(EnterWorldResult {
        account: account.username,
        character_name,
        realm_address: realm.address.clone(),
    })
}

async fn authenticate(
    stream: &mut TcpStream,
    username: &str,
    password: &str,
) -> anyhow::Result<([u8; SESSION_KEY_LEN], CMD_REALM_LIST_Server)> {
    CMD_AUTH_LOGON_CHALLENGE_Client {
        protocol_version: ProtocolVersion::Three,
        version: Version {
            major: 1,
            minor: 12,
            patch: 1,
            build: 5875,
        },
        platform: Platform::X86,
        os: Os::Windows,
        locale: Locale::EnGb,
        utc_timezone_offset: 0,
        client_ip_address: Ipv4Addr::LOCALHOST,
        account_name: username.to_string(),
    }
    .tokio_write(&mut *stream)
    .await?;

    let challenge =
        tokio_expect_login_server_message::<CMD_AUTH_LOGON_CHALLENGE_Server, _>(&mut *stream)
            .await?;
    let CMD_AUTH_LOGON_CHALLENGE_Server::Success {
        generator,
        large_safe_prime,
        salt,
        server_public_key,
        ..
    } = challenge
    else {
        anyhow::bail!("logon challenge rejected: {challenge:?}");
    };

    let client = SrpClientChallenge::new(
        NormalizedString::new(username)?,
        NormalizedString::new(password)?,
        generator[0],
        large_safe_prime
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid N"))?,
        PublicKey::from_le_bytes(server_public_key)?,
        salt,
    );

    CMD_AUTH_LOGON_PROOF_Client {
        client_public_key: *client.client_public_key(),
        client_proof: *client.client_proof(),
        crc_hash: [0; 20],
        telemetry_keys: vec![],
        security_flag: CMD_AUTH_LOGON_PROOF_Client_SecurityFlag::None,
    }
    .tokio_write(&mut *stream)
    .await?;

    let proof =
        tokio_expect_login_server_message::<CMD_AUTH_LOGON_PROOF_Server, _>(&mut *stream).await?;
    let CMD_AUTH_LOGON_PROOF_Server::Success { server_proof, .. } = proof else {
        anyhow::bail!("logon proof rejected: {proof:?}");
    };
    let client = client.verify_server_proof(server_proof)?;

    CMD_REALM_LIST_Client {}.tokio_write(&mut *stream).await?;
    let realms =
        tokio_expect_login_server_message::<CMD_REALM_LIST_Server, _>(&mut *stream).await?;

    Ok((*client.session_key(), realms))
}

async fn world_login(
    stream: &mut TcpStream,
    username: &str,
    session_key: [u8; SESSION_KEY_LEN],
    server_id: u8,
) -> anyhow::Result<String> {
    let challenge = tokio_expect_server_message::<SMSG_AUTH_CHALLENGE, _>(&mut *stream).await?;
    let seed = ProofSeed::new();
    let client_seed = seed.seed();
    let (client_proof, mut crypto) = seed.into_client_header_crypto(
        &NormalizedString::new(username)?,
        session_key,
        challenge.server_seed,
    );

    CMSG_AUTH_SESSION {
        build: 5875,
        server_id: u32::from(server_id),
        username: username.to_string(),
        client_seed,
        client_proof,
        addon_info: vec![AddonInfo {
            addon_name: "Test".to_string(),
            addon_crc: 0,
            addon_extra_crc: 0,
            addon_has_signature: 0,
        }],
    }
    .tokio_write_unencrypted_client(&mut *stream)
    .await?;

    let response = tokio_expect_server_message_encryption::<SMSG_AUTH_RESPONSE, _>(
        &mut *stream,
        crypto.decrypter(),
    )
    .await?;
    if !matches!(response, SMSG_AUTH_RESPONSE::AuthOk { .. }) {
        anyhow::bail!("world auth failed: {response:?}");
    }

    CMSG_CHAR_ENUM {}
        .tokio_write_encrypted_client(&mut *stream, crypto.encrypter())
        .await?;

    // The server may send SMSG_ADDON_INFO before CHAR_ENUM; skip until we see characters.
    let characters = loop {
        let opcode =
            ServerOpcodeMessage::tokio_read_encrypted(&mut *stream, crypto.decrypter()).await?;
        if let ServerOpcodeMessage::SMSG_CHAR_ENUM(message) = opcode {
            break message.characters;
        }
    };

    let character = characters
        .first()
        .ok_or_else(|| anyhow::anyhow!("character list is empty"))?;
    let name = character.name.clone();

    CMSG_PLAYER_LOGIN {
        guid: character.guid,
    }
    .tokio_write_encrypted_client(&mut *stream, crypto.encrypter())
    .await?;

    loop {
        let opcode =
            ServerOpcodeMessage::tokio_read_encrypted(&mut *stream, crypto.decrypter()).await?;
        if matches!(opcode, ServerOpcodeMessage::SMSG_LOGIN_VERIFY_WORLD(_)) {
            return Ok(name);
        }
    }
}
