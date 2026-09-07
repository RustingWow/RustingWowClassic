use tokio::net::TcpStream;
use wow_shared::{Account, CharacterTemplate, SessionInfo};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::{HeaderCrypto, ProofSeed};
use wow_world_messages::vanilla::{
    Addon, Addon_InfoBlock, Addon_UrlInfo, AddonType, CMSG_AUTH_SESSION, SMSG_ADDON_INFO,
    SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, ServerMessage, tokio_expect_client_message,
};

use crate::map_handle::MapHandle;
use crate::player::Player;
use crate::protocol::action::ClientAction;
use crate::protocol::{ClientConnection, ConnectionEvent};
use crate::router::{MapBackend, MapRouter, WorldPresence};
use crate::rpc::MapSession;
use crate::world::{ChatChannel, PlayerMailbox};

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
) -> anyhow::Result<()> {
    let Some(authenticated) = authenticate(stream, &auth_internal_url).await? else {
        return Ok(());
    };
    tracing::info!(account = %authenticated.account.username, "world session authenticated");

    let character = CharacterTemplate::for_account(&authenticated.account);
    let connection = ClientConnection::new(authenticated.stream, authenticated.encryption);
    run_session(connection, character, router, log_unhandled_packets).await
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
    mut character: CharacterTemplate,
    router: MapRouter,
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
                    &router,
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
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let in_world = presence.is_some();
    match action {
        ClientAction::Ping { sequence_id } => connection.pong(sequence_id).await?,
        ClientAction::ListCharacters => connection.send_character_list(character).await?,
        ClientAction::EnterWorld { guid } => {
            enter_world(connection, character, router, mailbox, presence, guid).await?;
        }
        ClientAction::QueryName { guid } => {
            reply_name(connection, character, router, presence, guid).await?;
        }
        ClientAction::QueryCreature { entry, guid } => {
            reply_creature(connection, presence, entry, guid).await?;
        }
        ClientAction::Select => {}
        ClientAction::Attack { guid } if in_world => {
            dispatch_attack(presence, mailbox, character.guid, guid).await?;
        }
        ClientAction::Attack { .. } => {}
        ClientAction::StopAttack if in_world => {
            dispatch_stop_attack(presence, mailbox, character.guid).await?;
        }
        ClientAction::StopAttack => {}
        ClientAction::ChangeStandState { state } if in_world => {
            dispatch_stand(presence, mailbox, character.guid, state).await?;
        }
        ClientAction::ChangeStandState { .. } => {}
        ClientAction::GossipHello { guid } if in_world => {
            dispatch_gossip_hello(presence, mailbox, character.guid, guid).await?;
        }
        ClientAction::GossipHello { .. } => {}
        ClientAction::GossipSelect { guid, option } if in_world => {
            dispatch_gossip_select(presence, mailbox, character.guid, guid, option).await?;
        }
        ClientAction::GossipSelect { .. } => {}
        ClientAction::QueryNpcText { text_id } => {
            let text = npc_text(presence, text_id).await?;
            connection.reply_npc_text(text_id, text.as_deref()).await?;
        }
        ClientAction::Moved(pending) if in_world => {
            if let Some(movement) = pending.bind(character.guid) {
                character.position = movement.position;
                dispatch_move(presence, mailbox, movement).await?;
            }
        }
        ClientAction::Moved(_) => {}
        ClientAction::Chat(pending) if in_world => {
            let chat = pending.bind(character.guid);
            match &chat.channel {
                ChatChannel::Whisper { to } => {
                    router
                        .directory()
                        .whisper(mailbox, chat.speaker, to.clone(), chat.text);
                }
                _ => dispatch_speak(presence, mailbox, chat).await?,
            }
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
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    guid: u64,
) -> anyhow::Result<()> {
    if guid != character.guid {
        tracing::warn!(guid, "unknown character guid");
        return Ok(());
    }

    connection.enter_world(character).await?;
    let player = Player::from(character);
    let map_id = character.map_id;
    let (others, creatures, backend) = open_map(router, map_id, player, mailbox.clone()).await?;
    *presence = Some(WorldPresence::new(
        router.directory().clone(),
        character.guid,
        mailbox.clone(),
        backend,
    ));
    connection.show_players(&others).await?;
    connection.show_creatures(&creatures).await?;
    tracing::info!(
        name = %character.name,
        map_id = presence.as_ref().map(WorldPresence::map_id),
        nearby = others.len(),
        npcs = creatures.len(),
        "player entered world"
    );
    Ok(())
}

async fn open_map(
    router: &MapRouter,
    map_id: u32,
    player: Player,
    mailbox: PlayerMailbox,
) -> anyhow::Result<(Vec<Player>, Vec<crate::creature::Creature>, MapBackend)> {
    if let Some(world) = router.map(map_id) {
        let world = world.clone();
        let result = router
            .join(map_id, player, mailbox)
            .ok_or_else(|| anyhow::anyhow!("map {map_id} missing"))?;
        return Ok((result.others, result.creatures, MapBackend::Local(world)));
    }
    let addr = router
        .endpoint(map_id)
        .ok_or_else(|| anyhow::anyhow!("no shard for map {map_id}"))?;
    let session = MapSession::connect(addr, map_id, mailbox.clone()).await?;
    let (others, creatures) = session.join(player.clone()).await?;
    router
        .directory()
        .register(player.guid, player.name, map_id, mailbox);
    Ok((others, creatures, MapBackend::Remote(session)))
}

async fn reply_name(
    connection: &mut ClientConnection,
    character: &CharacterTemplate,
    router: &MapRouter,
    presence: &Option<WorldPresence>,
    guid: u64,
) -> anyhow::Result<()> {
    if let Some(player) = lookup_player(presence, guid).await? {
        return connection.reply_name(guid, &player).await;
    }
    if let Some(name) = router.directory().name(guid) {
        let player = Player::new(guid, name, character.position);
        return connection.reply_name(guid, &player).await;
    }
    if guid == character.guid {
        return connection.reply_name(guid, &Player::from(character)).await;
    }
    Ok(())
}

async fn lookup_player(
    presence: &Option<WorldPresence>,
    guid: u64,
) -> anyhow::Result<Option<Player>> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        MapBackend::Local(world) => Ok(MapHandle::player(world, guid)),
        MapBackend::Remote(session) => session.player(guid).await,
    }
}

async fn reply_creature(
    connection: &mut ClientConnection,
    presence: &Option<WorldPresence>,
    entry: u32,
    guid: u64,
) -> anyhow::Result<()> {
    let creature = lookup_creature(presence, entry, guid).await?;
    let creature = creature.filter(|creature| creature.entry == entry);
    connection.reply_creature(entry, creature.as_ref()).await
}

async fn lookup_creature(
    presence: &Option<WorldPresence>,
    entry: u32,
    guid: u64,
) -> anyhow::Result<Option<crate::creature::Creature>> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        MapBackend::Local(world) => {
            Ok(MapHandle::creature(world, guid)
                .or_else(|| MapHandle::creature_by_entry(world, entry)))
        }
        MapBackend::Remote(session) => {
            if let Some(creature) = session.creature(guid).await? {
                return Ok(Some(creature));
            }
            session.creature_by_entry(entry).await
        }
    }
}

async fn npc_text(
    presence: &Option<WorldPresence>,
    text_id: u32,
) -> anyhow::Result<Option<String>> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        MapBackend::Local(world) => Ok(MapHandle::npc_text(world, text_id)),
        MapBackend::Remote(session) => session.npc_text(text_id).await,
    }
}

async fn dispatch_move(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    movement: crate::world::Movement,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::broadcast_move(world, mailbox, movement),
        MapBackend::Remote(session) => {
            session
                .broadcast_move(movement.guid, movement.position)
                .await?;
        }
    }
    Ok(())
}

async fn dispatch_speak(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    chat: crate::world::Chat,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::speak(world, mailbox, chat),
        MapBackend::Remote(session) => session.speak(chat).await?,
    }
    Ok(())
}

async fn dispatch_attack(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    attacker: u64,
    target: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::start_attack(world, mailbox, attacker, target),
        MapBackend::Remote(session) => session.start_attack(attacker, target).await?,
    }
    Ok(())
}

async fn dispatch_stop_attack(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    attacker: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::stop_attack(world, mailbox, attacker),
        MapBackend::Remote(session) => session.stop_attack(attacker).await?,
    }
    Ok(())
}

async fn dispatch_stand(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    guid: u64,
    state: u8,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::change_stand_state(world, mailbox, guid, state),
        MapBackend::Remote(session) => session.change_stand_state(guid, state).await?,
    }
    Ok(())
}

async fn dispatch_gossip_hello(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
    npc: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::open_gossip(world, mailbox, player, npc),
        MapBackend::Remote(session) => session.open_gossip(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_gossip_select(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
    npc: u64,
    option: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => {
            MapHandle::select_gossip_option(world, mailbox, player, npc, option)
        }
        MapBackend::Remote(session) => session.select_gossip_option(player, npc, option).await?,
    }
    Ok(())
}
