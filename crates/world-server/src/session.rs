use std::time::Duration;

use tokio::net::TcpStream;
use wow_shared::{
    Account, Appearance, CharacterClass, CharacterGender, CharacterMap, CharacterRace,
    CharacterTemplate, DbEnum, SessionInfo,
};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::{HeaderCrypto, ProofSeed};
use wow_world_messages::vanilla::{
    Addon, Addon_InfoBlock, Addon_UrlInfo, AddonType, CMSG_AUTH_SESSION, SMSG_ADDON_INFO,
    SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, ServerMessage, WorldResult,
    tokio_expect_client_message,
};

use crate::character_store::{CharacterDraft, CharacterStore, CreateCharacterError};
use crate::map_handle::MapHandle;
use crate::player::Player;
use crate::protocol::action::ClientAction;
use crate::protocol::{ClientConnection, ConnectionEvent};
use crate::router::{MapBackend, MapRouter, WorldPresence};
use crate::rpc::MapSession;
use crate::world::{ChatChannel, PlayerMailbox};

const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(60);

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
) -> anyhow::Result<()> {
    let mailbox = connection.mailbox();
    let mut presence: Option<WorldPresence> = None;
    let mut character: Option<CharacterTemplate> = None;
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
                ConnectionEvent::World(event) => {
                    if let Err(error) = connection.apply(event).await {
                        break Err(error);
                    }
                }
                ConnectionEvent::Action(action) => {
                    if let Err(error) = handle_action(
                        action,
                        &mut connection,
                        &account,
                        &characters,
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
            },
            _ = autosave.tick() => {
                if let Err(error) = persist_position(&characters, character.as_ref(), presence.as_ref()).await
                {
                    tracing::warn!(account = %account.username, %error, "character autosave failed");
                }
            }
        }
    };

    let save = persist_position(&characters, character.as_ref(), presence.as_ref()).await;
    drop(presence);
    connection.close();
    result.and(save)
}

async fn persist_position(
    store: &CharacterStore,
    character: Option<&CharacterTemplate>,
    presence: Option<&WorldPresence>,
) -> anyhow::Result<()> {
    let (Some(character), Some(presence)) = (character, presence) else {
        return Ok(());
    };
    let map_id = CharacterMap::from_protocol(presence.map_id())
        .ok_or_else(|| anyhow::anyhow!("unknown map {}", presence.map_id()))?;
    store
        .save_position(character.guid, map_id, character.position)
        .await
}

async fn handle_action(
    action: ClientAction,
    connection: &mut ClientConnection,
    account: &Account,
    store: &CharacterStore,
    character: &mut Option<CharacterTemplate>,
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    log_unhandled_packets: bool,
) -> anyhow::Result<()> {
    let in_world = presence.is_some();
    match action {
        ClientAction::Ping { sequence_id } => connection.pong(sequence_id).await?,
        ClientAction::ListCharacters => {
            let list = store.list(account.id).await?;
            connection.send_character_list(&list).await?;
        }
        ClientAction::CreateCharacter {
            name,
            race,
            class,
            gender,
            appearance,
        } => {
            create_character(
                connection, store, account.id, name, race, class, gender, appearance,
            )
            .await?;
        }
        ClientAction::CreateCharacterRejected => {
            connection
                .char_create(WorldResult::CharCreateDisabled)
                .await?;
        }
        ClientAction::DeleteCharacter { guid } => {
            delete_character(connection, store, account.id, guid, character, presence).await?;
        }
        ClientAction::EnterWorld { guid } => {
            enter_world(
                connection, store, account, character, router, mailbox, presence, guid,
            )
            .await?;
        }
        ClientAction::LogoutRequest | ClientAction::PlayerLogout => {
            leave_world(connection, store, character, presence).await?;
        }
        ClientAction::LogoutCancel => connection.logout_cancel_ack().await?,
        ClientAction::QueryName { guid } => {
            reply_name(
                connection,
                store,
                character.as_ref(),
                router,
                presence,
                guid,
            )
            .await?;
        }
        ClientAction::QueryCreature { entry, guid } => {
            reply_creature(connection, presence, entry, guid).await?;
        }
        ClientAction::Select => {}
        ClientAction::Attack { guid } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_attack(presence, mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::Attack { .. } => {}
        ClientAction::StopAttack if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_stop_attack(presence, mailbox, character.guid).await?;
            }
        }
        ClientAction::StopAttack => {}
        ClientAction::ChangeStandState { state } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_stand(presence, mailbox, character.guid, state).await?;
            }
        }
        ClientAction::ChangeStandState { .. } => {}
        ClientAction::GossipHello { guid } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_gossip_hello(presence, mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::GossipHello { .. } => {}
        ClientAction::GossipSelect { guid, option } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_gossip_select(presence, mailbox, character.guid, guid, option).await?;
            }
        }
        ClientAction::GossipSelect { .. } => {}
        ClientAction::QueryNpcText { text_id } => {
            let text = npc_text(presence, text_id).await?;
            connection.reply_npc_text(text_id, text.as_deref()).await?;
        }
        ClientAction::ListVendor { guid } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_list_vendor(presence, mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::ListVendor { .. } => {}
        ClientAction::BuyItem {
            vendor,
            item,
            amount,
        } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_buy_item(presence, mailbox, character.guid, vendor, item, amount).await?;
            }
        }
        ClientAction::BuyItem { .. } => {}
        ClientAction::QueryItem { entry } => {
            let item = lookup_item(presence, entry).await?;
            connection.reply_item(entry, item.as_ref()).await?;
        }
        ClientAction::Loot { guid } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_loot(presence, mailbox, character.guid, guid).await?;
            }
        }
        ClientAction::Loot { .. } => {}
        ClientAction::LootItem { index } if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_take_loot(presence, mailbox, character.guid, index).await?;
            }
        }
        ClientAction::LootItem { .. } => {}
        ClientAction::LootRelease if in_world => {
            if let Some(character) = character.as_ref() {
                dispatch_close_loot(presence, mailbox, character.guid).await?;
            }
        }
        ClientAction::LootRelease => {}
        ClientAction::Moved(pending) if in_world => {
            if let Some(character) = character.as_mut() {
                if let Some(movement) = pending.bind(character.guid) {
                    character.position = movement.position;
                    dispatch_move(presence, mailbox, movement).await?;
                }
            }
        }
        ClientAction::Moved(_) => {}
        ClientAction::Chat(pending) if in_world => {
            if let Some(character) = character.as_ref() {
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

async fn create_character(
    connection: &mut ClientConnection,
    store: &CharacterStore,
    account_id: i64,
    name: String,
    race: CharacterRace,
    class: CharacterClass,
    gender: CharacterGender,
    appearance: Appearance,
) -> anyhow::Result<()> {
    let result = match store
        .create(CharacterDraft {
            account_id,
            name,
            race,
            class,
            gender,
            appearance,
        })
        .await
    {
        Ok(_) => WorldResult::CharCreateSuccess,
        Err(CreateCharacterError::InvalidName) => WorldResult::CharCreateError,
        Err(CreateCharacterError::NameInUse) => WorldResult::CharCreateNameInUse,
        Err(CreateCharacterError::AccountLimit) => WorldResult::CharCreateAccountLimit,
        Err(CreateCharacterError::Disabled) => WorldResult::CharCreateDisabled,
        Err(CreateCharacterError::Store(error)) => {
            tracing::warn!(%error, "character create failed");
            WorldResult::CharCreateError
        }
    };
    connection.char_create(result).await
}

async fn delete_character(
    connection: &mut ClientConnection,
    store: &CharacterStore,
    account_id: i64,
    guid: u64,
    character: &Option<CharacterTemplate>,
    presence: &Option<WorldPresence>,
) -> anyhow::Result<()> {
    let in_world = presence.is_some() && character.as_ref().is_some_and(|c| c.guid == guid);
    let result = if in_world {
        WorldResult::CharDeleteFailed
    } else if store.delete(account_id, guid).await? {
        WorldResult::CharDeleteSuccess
    } else {
        WorldResult::CharDeleteFailed
    };
    connection.char_delete(result).await
}

async fn leave_world(
    connection: &mut ClientConnection,
    store: &CharacterStore,
    character: &mut Option<CharacterTemplate>,
    presence: &mut Option<WorldPresence>,
) -> anyhow::Result<()> {
    persist_position(store, character.as_ref(), presence.as_ref()).await?;
    *presence = None;
    *character = None;
    connection.logout_to_character_screen().await
}

async fn enter_world(
    connection: &mut ClientConnection,
    store: &CharacterStore,
    account: &Account,
    character: &mut Option<CharacterTemplate>,
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    guid: u64,
) -> anyhow::Result<()> {
    if presence.is_some() {
        return Ok(());
    }
    let Some(loaded) = store.get(account.id, guid).await? else {
        tracing::warn!(guid, account = %account.username, "unknown character guid");
        return Ok(());
    };

    connection.enter_world(&loaded).await?;
    let player = Player::from(&loaded);
    let map_id = loaded.map_id.as_protocol();
    let (others, creatures, backend) = open_map(router, map_id, player, mailbox.clone()).await?;
    *presence = Some(WorldPresence::new(
        router.directory().clone(),
        loaded.guid,
        mailbox.clone(),
        backend,
    ));
    connection.show_players(&others).await?;
    connection.show_creatures(&creatures).await?;
    tracing::info!(
        name = %loaded.name,
        map_id = presence.as_ref().map(WorldPresence::map_id),
        nearby = others.len(),
        npcs = creatures.len(),
        "player entered world"
    );
    store.mark_entered_world(loaded.guid).await?;
    *character = Some(loaded);
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
    store: &CharacterStore,
    character: Option<&CharacterTemplate>,
    router: &MapRouter,
    presence: &Option<WorldPresence>,
    guid: u64,
) -> anyhow::Result<()> {
    if let Some(player) = lookup_player(presence, guid).await? {
        return connection.reply_name(guid, &player).await;
    }
    if let Some(stored) = store.get_by_guid(guid).await? {
        return connection.reply_name(guid, &Player::from(&stored)).await;
    }
    if let Some(name) = router.directory().name(guid) {
        let position = character
            .map(|character| character.position)
            .unwrap_or(wow_shared::Position::NORTHSHIRE);
        let player = Player::new(guid, name, position);
        return connection.reply_name(guid, &player).await;
    }
    if let Some(character) = character.filter(|character| character.guid == guid) {
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

async fn dispatch_list_vendor(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
    npc: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::list_vendor(world, mailbox, player, npc),
        MapBackend::Remote(session) => session.list_vendor(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_buy_item(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
    npc: u64,
    item: u32,
    amount: u32,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::buy_item(world, mailbox, player, npc, item, amount),
        MapBackend::Remote(session) => session.buy_item(player, npc, item, amount).await?,
    }
    Ok(())
}

async fn dispatch_loot(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
    npc: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::open_loot(world, mailbox, player, npc),
        MapBackend::Remote(session) => session.open_loot(player, npc).await?,
    }
    Ok(())
}

async fn dispatch_take_loot(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
    index: u8,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::take_loot(world, mailbox, player, 0, index),
        MapBackend::Remote(session) => session.take_loot(player, index).await?,
    }
    Ok(())
}

async fn dispatch_close_loot(
    presence: &Option<WorldPresence>,
    mailbox: &PlayerMailbox,
    player: u64,
) -> anyhow::Result<()> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(());
    };
    match backend {
        MapBackend::Local(world) => MapHandle::close_loot(world, mailbox, player),
        MapBackend::Remote(session) => session.close_loot(player).await?,
    }
    Ok(())
}

async fn lookup_item(
    presence: &Option<WorldPresence>,
    entry: u32,
) -> anyhow::Result<Option<crate::catalog::ItemRow>> {
    let Some(backend) = presence.as_ref().and_then(WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        MapBackend::Local(world) => Ok(MapHandle::item(world, entry)),
        MapBackend::Remote(session) => session.item(entry).await,
    }
}
