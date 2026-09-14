use tokio::net::{TcpListener, TcpStream};
use wow_login_messages::Message;
use wow_login_messages::all::{CMD_AUTH_LOGON_CHALLENGE_Client, ProtocolVersion};
use wow_login_messages::errors::ExpectedOpcodeError;
use wow_login_messages::helper::{
    InitialMessage, tokio_expect_client_message, tokio_read_initial_message,
};
use wow_login_messages::version_3::{
    CMD_AUTH_LOGON_CHALLENGE_Server, CMD_AUTH_LOGON_CHALLENGE_Server_SecurityFlag,
    CMD_AUTH_LOGON_PROOF_Client, CMD_AUTH_LOGON_PROOF_Server, CMD_REALM_LIST_Client,
    CMD_REALM_LIST_Server, Realm, RealmFlag, RealmType,
};
use wow_shared::SessionInfo;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::{SrpProof, SrpVerifier};
use wow_srp::{GENERATOR, LARGE_SAFE_PRIME_LITTLE_ENDIAN, PublicKey};

use crate::accounts::{AccountRecord, AccountStore};
use crate::store::SessionStore;

pub async fn accept_loop(
    listener: TcpListener,
    store: SessionStore,
    accounts: AccountStore,
    world_public_addr: String,
) -> anyhow::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let store = store.clone();
        let accounts = accounts.clone();
        let world_public_addr = world_public_addr.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_client(stream, store, accounts, world_public_addr).await {
                tracing::warn!(%peer, %error, "auth session ended");
            }
        });
    }
}

async fn handle_client(
    mut stream: TcpStream,
    store: SessionStore,
    accounts: AccountStore,
    world_public_addr: String,
) -> anyhow::Result<()> {
    let message = match tokio_read_initial_message(&mut stream).await {
        Ok(message) => message,
        Err(ExpectedOpcodeError::Io(error)) => return Err(error.into()),
        Err(error) => {
            tracing::debug!(%error, "invalid initial auth opcode");
            return Ok(());
        }
    };

    match message {
        InitialMessage::Logon(challenge) => {
            login(stream, challenge, store, accounts, world_public_addr).await
        }
        InitialMessage::Reconnect(_) => {
            tracing::debug!("reconnect is not implemented");
            Ok(())
        }
    }
}

async fn login(
    mut stream: TcpStream,
    challenge: CMD_AUTH_LOGON_CHALLENGE_Client,
    store: SessionStore,
    accounts: AccountStore,
    world_public_addr: String,
) -> anyhow::Result<()> {
    if !matches!(
        challenge.protocol_version,
        ProtocolVersion::Two | ProtocolVersion::Three
    ) {
        CMD_AUTH_LOGON_CHALLENGE_Server::FailVersionInvalid
            .tokio_write(&mut stream)
            .await?;
        return Ok(());
    }

    let account = match accounts.get_by_username(&challenge.account_name).await? {
        Some(account) if account.locked => {
            tracing::info!(account = %account.username, "locked account");
            CMD_AUTH_LOGON_CHALLENGE_Server::FailBanned
                .tokio_write(&mut stream)
                .await?;
            return Ok(());
        }
        Some(account) => account,
        None => {
            tracing::info!(account = %challenge.account_name, "unknown account");
            CMD_AUTH_LOGON_CHALLENGE_Server::FailUnknownAccount
                .tokio_write(&mut stream)
                .await?;
            return Ok(());
        }
    };

    let proof = srp_proof(&account)?;
    CMD_AUTH_LOGON_CHALLENGE_Server::Success {
        server_public_key: *proof.server_public_key(),
        generator: vec![GENERATOR],
        large_safe_prime: LARGE_SAFE_PRIME_LITTLE_ENDIAN.into(),
        salt: *proof.salt(),
        crc_salt: [0; 16],
        security_flag: CMD_AUTH_LOGON_CHALLENGE_Server_SecurityFlag::None,
    }
    .tokio_write(&mut stream)
    .await?;
    tracing::info!(account = %account.username, "sent logon challenge");

    let client_proof =
        tokio_expect_client_message::<CMD_AUTH_LOGON_PROOF_Client, _>(&mut stream).await?;

    let (server, server_proof) = match proof.into_server(
        PublicKey::from_le_bytes(client_proof.client_public_key)?,
        client_proof.client_proof,
    ) {
        Ok(ok) => ok,
        Err(error) => {
            tracing::info!(account = %account.username, %error, "invalid password proof");
            CMD_AUTH_LOGON_PROOF_Server::FailIncorrectPassword
                .tokio_write(&mut stream)
                .await?;
            return Ok(());
        }
    };

    CMD_AUTH_LOGON_PROOF_Server::Success {
        server_proof,
        hardware_survey_id: 0,
    }
    .tokio_write(&mut stream)
    .await?;

    store.insert(SessionInfo {
        account_id: account.id,
        account: account.username.clone(),
        session_key: *server.session_key(),
        gmlevel: account.gmlevel,
    });
    tracing::info!(account = %account.username, "authenticated");

    send_realm_list(&mut stream, &world_public_addr).await
}

fn srp_proof(account: &AccountRecord) -> anyhow::Result<SrpProof> {
    let username = NormalizedString::new(&account.username)?;
    Ok(SrpVerifier::from_database_values(username, account.verifier, account.salt).into_proof())
}

async fn send_realm_list(stream: &mut TcpStream, world_public_addr: &str) -> anyhow::Result<()> {
    while tokio_expect_client_message::<CMD_REALM_LIST_Client, _>(&mut *stream)
        .await
        .is_ok()
    {
        CMD_REALM_LIST_Server {
            realms: vec![Realm {
                realm_type: RealmType::PlayerVsEnvironment,
                flag: RealmFlag::empty(),
                name: "WoWServer".to_string(),
                address: world_public_addr.to_string(),
                population: Default::default(),
                number_of_characters_on_realm: 1,
                category: Default::default(),
                realm_id: 1,
            }],
        }
        .tokio_write(&mut *stream)
        .await?;
        tracing::info!("sent realm list");
    }
    Ok(())
}
