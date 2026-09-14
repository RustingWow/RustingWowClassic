use wow_shared::{
    Appearance, CharacterClass, CharacterGender, CharacterMap, CharacterRace, CharacterTemplate,
    DbEnum,
};
use wow_world_messages::vanilla::WorldResult;

use crate::character_store::{CharacterDraft, CreateCharacterError};
use crate::player::Player;
use crate::protocol::ClientConnection;
use crate::protocol::action::ClientAction;
use crate::router::MapRouter;
use crate::rpc::MapSession;
use crate::world::PlayerMailbox;

use super::{ActionContext, GmSession, MapBackend, WorldPresence};

pub(super) async fn handle(
    action: ClientAction,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::Ping { sequence_id } => ctx.connection.pong(sequence_id).await?,
        ClientAction::ListCharacters => {
            let list = ctx.store.list(ctx.account.id).await?;
            ctx.connection.send_character_list(&list).await?;
        }
        ClientAction::CreateCharacter {
            name,
            race,
            class,
            gender,
            appearance,
        } => {
            create_character(
                ctx.connection,
                ctx.store,
                ctx.account.id,
                name,
                race,
                class,
                gender,
                appearance,
            )
            .await?;
        }
        ClientAction::CreateCharacterRejected => {
            ctx.connection
                .char_create(WorldResult::CharCreateDisabled)
                .await?;
        }
        ClientAction::DeleteCharacter { guid } => {
            delete_character(
                ctx.connection,
                ctx.store,
                ctx.account.id,
                guid,
                ctx.character,
                ctx.presence,
            )
            .await?;
        }
        ClientAction::EnterWorld { guid } => {
            enter_world(
                ctx.connection,
                ctx.store,
                ctx.account,
                ctx.character,
                ctx.router,
                ctx.mailbox,
                ctx.presence,
                guid,
                ctx.gm,
            )
            .await?;
        }
        ClientAction::LogoutRequest | ClientAction::PlayerLogout => {
            leave_world(ctx.connection, ctx.store, ctx.character, ctx.presence).await?;
        }
        ClientAction::LogoutCancel => ctx.connection.logout_cancel_ack().await?,
        _ => unreachable!("character::handle called with non-character action"),
    }
    Ok(true)
}

pub(crate) async fn persist_position(
    store: &crate::character_store::CharacterStore,
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

async fn create_character(
    connection: &mut ClientConnection,
    store: &crate::character_store::CharacterStore,
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
    store: &crate::character_store::CharacterStore,
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

pub(crate) async fn leave_world(
    connection: &mut ClientConnection,
    store: &crate::character_store::CharacterStore,
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
    store: &crate::character_store::CharacterStore,
    account: &wow_shared::Account,
    character: &mut Option<CharacterTemplate>,
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    guid: u64,
    gm: &GmSession,
) -> anyhow::Result<()> {
    if presence.is_some() {
        return Ok(());
    }
    let Some(loaded) = store.get(account.id, guid).await? else {
        tracing::warn!(guid, account = %account.username, "unknown character guid");
        return Ok(());
    };

    let mut player = Player::from(&loaded);
    player.gmlevel = account.gmlevel;
    player.gm_on = gm.on;
    player.gm_visible = gm.visible;
    player.gm_chat = gm.chat;
    let (quest_log, rewarded) = store.load_quests(guid).await?;
    player.quest_log = quest_log;
    player.rewarded_quests = rewarded;
    connection.enter_world(&loaded, &player).await?;
    let map_id = loaded.map_id.as_protocol();
    let (others, creatures, gameobjects, backend) =
        open_map(router, map_id, player, mailbox.clone()).await?;
    router.directory().set_gmlevel(loaded.guid, account.gmlevel);
    *presence = Some(WorldPresence::new(
        router.directory().clone(),
        loaded.guid,
        mailbox.clone(),
        backend,
    ));
    connection.show_players(&others).await?;
    connection.show_creatures(&creatures).await?;
    connection.show_gameobjects(&gameobjects).await?;
    tracing::info!(
        name = %loaded.name,
        map_id = presence.as_ref().map(WorldPresence::map_id),
        nearby = others.len(),
        npcs = creatures.len(),
        gameobjects = gameobjects.len(),
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
) -> anyhow::Result<(
    Vec<Player>,
    Vec<crate::creature::Creature>,
    Vec<crate::gameobject::GameObject>,
    MapBackend,
)> {
    if let Some(world) = router.map(map_id) {
        let world = world.clone();
        let result = router
            .join(map_id, player, mailbox)
            .ok_or_else(|| anyhow::anyhow!("map {map_id} missing"))?;
        return Ok((
            result.others,
            result.creatures,
            result.gameobjects,
            MapBackend::Local(world),
        ));
    }
    let addr = router
        .endpoint(map_id)
        .ok_or_else(|| anyhow::anyhow!("no shard for map {map_id}"))?;
    let session = MapSession::connect(addr, map_id, mailbox.clone()).await?;
    let (others, creatures, gameobjects) = session.join(player.clone()).await?;
    router
        .directory()
        .register(player.guid, player.name, map_id, mailbox);
    Ok((others, creatures, gameobjects, MapBackend::Remote(session)))
}

pub(crate) async fn teleport_in_world(
    connection: &mut ClientConnection,
    store: &crate::character_store::CharacterStore,
    character: &mut CharacterTemplate,
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    account: &wow_shared::Account,
    gm: &mut GmSession,
    map_id: u32,
    position: wow_shared::Position,
) -> anyhow::Result<()> {
    let Some(presence_ref) = presence.as_mut() else {
        return Ok(());
    };
    let from_map = presence_ref.map_id();
    gm.recall = Some((from_map, character.position));
    character.position = position;
    if let Some(map) = CharacterMap::from_protocol(map_id) {
        character.map_id = map;
    }
    connection
        .transfer_world(
            CharacterMap::from_protocol(map_id).unwrap_or(character.map_id),
            position,
        )
        .await?;
    if from_map == map_id {
        if let Some(MapBackend::Local(world)) = presence_ref.backend() {
            world.relocate(character.guid, position);
        }
        return Ok(());
    }
    let current = match presence_ref.backend() {
        Some(MapBackend::Local(world)) => world.player(character.guid),
        Some(MapBackend::Remote(session)) => session.player(character.guid).await?,
        None => None,
    };
    let mut player = Player::from(&*character);
    player.gmlevel = account.gmlevel;
    player.gm_on = gm.on;
    player.gm_visible = gm.visible;
    player.gm_chat = gm.chat;
    if let Some(current) = current {
        player.quest_log = current.quest_log;
        player.rewarded_quests = current.rewarded_quests;
    }
    let (others, creatures, gameobjects, backend) =
        open_map(router, map_id, player, mailbox.clone()).await?;
    presence_ref.set_backend(backend);
    router
        .directory()
        .set_gmlevel(character.guid, account.gmlevel);
    connection.show_players(&others).await?;
    connection.show_creatures(&creatures).await?;
    connection.show_gameobjects(&gameobjects).await?;
    store
        .save_position(character.guid, character.map_id, character.position)
        .await?;
    Ok(())
}
