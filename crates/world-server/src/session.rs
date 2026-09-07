use tokio::net::TcpStream;
use wow_shared::{Account, CharacterTemplate, SessionInfo, parse_account};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::{HeaderCrypto, ProofSeed};
use wow_world_messages::vanilla::{
    Addon, Addon_InfoBlock, Addon_UrlInfo, AddonType, CMSG_AUTH_SESSION, SMSG_ADDON_INFO,
    SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, ServerMessage, tokio_expect_client_message,
};

use crate::player::Player;
use crate::protocol::action::ClientAction;
use crate::protocol::{ClientConnection, ConnectionEvent};
use crate::world::{PlayerMailbox, World, WorldPresence};

struct AuthenticatedClient {
    stream: TcpStream,
    encryption: HeaderCrypto,
    account: Account,
}

pub async fn handle_client(
    stream: TcpStream,
    auth_internal_url: String,
    log_unhandled_packets: bool,
    world: World,
) -> anyhow::Result<()> {
    let Some(authenticated) = authenticate(stream, &auth_internal_url).await? else {
        return Ok(());
    };
    tracing::info!(account = %authenticated.account.username, "world session authenticated");

    let character = CharacterTemplate::for_account(&authenticated.account);
    let connection = ClientConnection::new(authenticated.stream, authenticated.encryption);
    run_session(connection, character, world, log_unhandled_packets).await
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
    let account = parse_account(&auth.username)?;
    let session = fetch_session(auth_internal_url, &account.username).await?;

    let mut encryption = match seed.into_server_header_crypto(
        &NormalizedString::new(&account.username)?,
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
    mut character: CharacterTemplate,
    world: World,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let mailbox = connection.mailbox();
    let mut presence: Option<WorldPresence> = None;

    let result = loop {
        match connection.next().await {
            ConnectionEvent::Disconnected => {
                tracing::debug!(name = %character.name, "client disconnected");
                break Ok(());
            }
            ConnectionEvent::World(event) => {
                if let Err(error) = connection.apply(event).await {
                    break Err(error);
                }
            }
            ConnectionEvent::Action(action) => {
                if let Err(error) = handle_action(
                    action,
                    &mut connection,
                    &mut character,
                    &world,
                    &mailbox,
                    &mut presence,
                    log_unhandled_packets,
                )
                .await
                {
                    break Err(error);
                }
            }
        }
    };

    drop(presence);
    connection.close();
    result
}

async fn handle_action(
    action: ClientAction,
    connection: &mut ClientConnection,
    character: &mut CharacterTemplate,
    world: &World,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let in_world = presence.is_some();
    match action {
        ClientAction::Ping { sequence_id } => connection.pong(sequence_id).await?,
        ClientAction::ListCharacters => connection.send_character_list(character).await?,
        ClientAction::EnterWorld { guid } => {
            enter_world(connection, character, world, mailbox, presence, guid).await?;
        }
        ClientAction::QueryName { guid } => reply_name(connection, character, world, guid).await?,
        ClientAction::QueryCreature { entry, guid } => {
            reply_creature(connection, world, entry, guid).await?;
        }
        ClientAction::Select => {}
        ClientAction::Attack { guid } if in_world => {
            world.start_attack(mailbox, character.guid, guid);
        }
        ClientAction::Attack { .. } => {}
        ClientAction::StopAttack if in_world => {
            world.stop_attack(mailbox, character.guid);
        }
        ClientAction::StopAttack => {}
        ClientAction::ChangeStandState { state } if in_world => {
            world.change_stand_state(mailbox, character.guid, state);
        }
        ClientAction::ChangeStandState { .. } => {}
        ClientAction::GossipHello { guid } if in_world => {
            world.open_gossip(mailbox, character.guid, guid);
        }
        ClientAction::GossipHello { .. } => {}
        ClientAction::GossipSelect { guid, option } if in_world => {
            world.select_gossip_option(mailbox, character.guid, guid, option);
        }
        ClientAction::GossipSelect { .. } => {}
        ClientAction::QueryNpcText { text_id } => {
            let text = world.npc_text(text_id);
            connection.reply_npc_text(text_id, text.as_deref()).await?;
        }
        ClientAction::Moved(pending) if in_world => {
            if let Some(movement) = pending.bind(character.guid) {
                character.position = movement.position;
                world.broadcast_move(mailbox, movement);
            }
        }
        ClientAction::Moved(_) => {}
        ClientAction::Chat(pending) if in_world => {
            world.speak(mailbox, pending.bind(character.guid));
        }
        ClientAction::Chat(_) => {}
        ClientAction::Ignored(ignored) => {
            if log_unhandled_packets {
                tracing::info!(
                    opcode = ignored.opcode,
                    name = ignored.name.as_deref(),
                    in_world,
                    "unhandled packet"
                );
            }
        }
    }
    Ok(())
}

async fn enter_world(
    connection: &mut ClientConnection,
    character: &CharacterTemplate,
    world: &World,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    guid: u64,
) -> anyhow::Result<()> {
    if guid != character.guid {
        tracing::warn!(guid, "unknown character guid");
        return Ok(());
    }

    connection.enter_world(character).await?;
    let nearby = world.join(Player::from(character), mailbox.clone());
    *presence = Some(WorldPresence::new(
        world.clone(),
        character.guid,
        mailbox.clone(),
    ));
    connection.show_players(&nearby).await?;
    let npcs = world.creatures();
    connection.show_creatures(&npcs).await?;
    tracing::info!(
        name = %character.name,
        nearby = nearby.len(),
        npcs = npcs.len(),
        "player entered world"
    );
    Ok(())
}

async fn reply_name(
    connection: &mut ClientConnection,
    character: &CharacterTemplate,
    world: &World,
    guid: u64,
) -> anyhow::Result<()> {
    let player = world
        .player(guid)
        .or_else(|| (guid == character.guid).then(|| Player::from(character)));
    let Some(player) = player else {
        return Ok(());
    };
    connection.reply_name(guid, &player).await
}

async fn reply_creature(
    connection: &mut ClientConnection,
    world: &World,
    entry: u32,
    guid: u64,
) -> anyhow::Result<()> {
    let creature = world
        .creature(guid)
        .or_else(|| world.creature_by_entry(entry));
    let creature = creature.filter(|creature| creature.entry == entry);
    connection.reply_creature(entry, creature.as_ref()).await
}
