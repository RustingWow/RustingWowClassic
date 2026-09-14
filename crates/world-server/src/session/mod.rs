mod character;
mod chat;
mod combat;
mod command;
mod dispatch;
mod gossip;
mod loot;
mod movement;
mod query;
mod quest;
mod select;

use std::time::Duration;

use tokio::net::TcpStream;
use wow_shared::{Account, CharacterTemplate, SessionInfo};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::{HeaderCrypto, ProofSeed};
use wow_world_messages::vanilla::{
    Addon, Addon_InfoBlock, Addon_UrlInfo, AddonType, CMSG_AUTH_SESSION, SMSG_ADDON_INFO,
    SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, ServerMessage, tokio_expect_client_message,
};

use crate::character_store::CharacterStore;
use crate::command::CommandServices;
use crate::protocol::{ClientConnection, ConnectionEvent};
use crate::router::MapRouter;
use crate::world::{PlayerMailbox, WorldEvent};

pub(crate) use crate::router::{MapBackend, WorldPresence};

const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) struct GmSession {
    pub on: bool,
    pub visible: bool,
    pub chat: bool,
    pub selection: Option<u64>,
    pub recall: Option<(u32, wow_shared::Position)>,
}

impl Default for GmSession {
    fn default() -> Self {
        Self {
            on: false,
            visible: true,
            chat: false,
            selection: None,
            recall: None,
        }
    }
}

pub(crate) struct ActionContext<'a> {
    pub connection: &'a mut ClientConnection,
    pub account: &'a Account,
    pub store: &'a CharacterStore,
    pub character: &'a mut Option<CharacterTemplate>,
    pub router: &'a MapRouter,
    pub mailbox: &'a PlayerMailbox,
    pub presence: &'a mut Option<WorldPresence>,
    pub log_unhandled_packets: bool,
    pub services: &'a CommandServices,
    pub gm: &'a mut GmSession,
}

struct AuthenticatedClient {
    stream: TcpStream,
    encryption: HeaderCrypto,
    account: Account,
}

pub async fn handle_client(
    stream: TcpStream,
    auth_internal_url: String,
    log_unhandled_packets: bool,
    router: MapRouter,
    characters: CharacterStore,
    services: CommandServices,
) -> anyhow::Result<()> {
    let Some(authenticated) = authenticate(stream, &auth_internal_url).await? else {
        return Ok(());
    };
    tracing::info!(account = %authenticated.account.username, "world session authenticated");

    let connection = ClientConnection::new(authenticated.stream, authenticated.encryption);
    run_session(
        connection,
        authenticated.account,
        characters,
        router,
        log_unhandled_packets,
        services,
    )
    .await
}

async fn authenticate(
    mut stream: TcpStream,
    auth_internal_url: &str,
) -> anyhow::Result<Option<AuthenticatedClient>> {
    let seed = ProofSeed::new();
    SMSG_AUTH_CHALLENGE {
        server_seed: seed.seed(),
    }
    .tokio_write_unencrypted_server(&mut stream)
    .await?;

    let auth = tokio_expect_client_message::<CMSG_AUTH_SESSION, _>(&mut stream).await?;
    let username = NormalizedString::new(&auth.username)?;
    let session = fetch_session(auth_internal_url, username.as_ref()).await?;
    let account = Account {
        id: session.account_id,
        username: session.account.clone(),
        gmlevel: session.gmlevel.min(3),
    };

    let mut encryption = match seed.into_server_header_crypto(
        &username,
        session.session_key,
        auth.client_proof,
        auth.client_seed,
    ) {
        Ok(crypto) => crypto,
        Err(error) => {
            tracing::info!(account = %account.username, %error, "world auth proof failed");
            return Ok(None);
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
    Ok(Some(AuthenticatedClient {
        stream,
        encryption,
        account,
    }))
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
    mut connection: ClientConnection,
    account: Account,
    characters: CharacterStore,
    router: MapRouter,
    log_unhandled_packets: bool,
    services: CommandServices,
) -> anyhow::Result<()> {
    let mailbox = connection.mailbox();
    let mut presence: Option<WorldPresence> = None;
    let mut character: Option<CharacterTemplate> = None;
    let mut gm = GmSession::default();
    let mut autosave = tokio::time::interval_at(
        tokio::time::Instant::now() + AUTOSAVE_INTERVAL,
        AUTOSAVE_INTERVAL,
    );

    let result = loop {
        tokio::select! {
            event = connection.next() => match event {
                ConnectionEvent::Disconnected => {
                    tracing::debug!(account = %account.username, "client disconnected");
                    break Ok(());
                }
                ConnectionEvent::World(WorldEvent::ForcedTeleport { map_id, position }) => {
                    if let Some(character) = character.as_mut()
                        && let Err(error) = character::teleport_in_world(
                            &mut connection,
                            &characters,
                            character,
                            &router,
                            &mailbox,
                            &mut presence,
                            &account,
                            &mut gm,
                            map_id,
                            position,
                        )
                        .await
                    {
                        break Err(error);
                    }
                }
                ConnectionEvent::World(WorldEvent::QuestStateChanged { guid, change }) => {
                    if let Err(error) = characters.apply_quest_change(guid, change).await {
                        tracing::warn!(account = %account.username, %error, "quest persist failed");
                    }
                }
                ConnectionEvent::World(event) => {
                    if let Err(error) = connection.apply(event).await {
                        break Err(error);
                    }
                }
                ConnectionEvent::Action(action) => {
                    let mut ctx = ActionContext {
                        connection: &mut connection,
                        account: &account,
                        store: &characters,
                        character: &mut character,
                        router: &router,
                        mailbox: &mailbox,
                        presence: &mut presence,
                        log_unhandled_packets,
                        services: &services,
                        gm: &mut gm,
                    };
                    match dispatch::handle_action(action, &mut ctx).await {
                        Ok(true) => {}
                        Ok(false) => break Ok(()),
                        Err(error) => break Err(error),
                    }
                }
            },
            _ = autosave.tick() => {
                if let Err(error) = character::persist_position(&characters, character.as_ref(), presence.as_ref()).await
                {
                    tracing::warn!(account = %account.username, %error, "character autosave failed");
                }
            }
        }
    };

    let save =
        character::persist_position(&characters, character.as_ref(), presence.as_ref()).await;
    drop(presence);
    connection.close();
    result.and(save)
}
