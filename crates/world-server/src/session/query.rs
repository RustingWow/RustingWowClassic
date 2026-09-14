use wow_shared::CharacterTemplate;

use crate::player::Player;
use crate::protocol::ClientConnection;
use crate::protocol::action::ClientAction;
use crate::router::MapRouter;

use super::ActionContext;

pub(super) async fn handle(
    action: ClientAction,
    ctx: &mut ActionContext<'_>,
) -> anyhow::Result<bool> {
    match action {
        ClientAction::QueryName { guid } => {
            reply_name(
                ctx.connection,
                ctx.store,
                ctx.character.as_ref(),
                ctx.router,
                ctx.presence,
                guid,
            )
            .await?;
        }
        ClientAction::QueryCreature { entry, guid } => {
            reply_creature(ctx.connection, ctx.presence, entry, guid).await?;
        }
        ClientAction::QueryGameObject { entry, guid } => {
            reply_gameobject(ctx.connection, ctx.presence, entry, guid).await?;
        }
        ClientAction::QueryItem { entry } => {
            let item = lookup_item(ctx.presence, entry).await?;
            ctx.connection.reply_item(entry, item.as_ref()).await?;
        }
        _ => unreachable!("query::handle called with non-query action"),
    }
    Ok(true)
}

async fn reply_name(
    connection: &mut ClientConnection,
    store: &crate::character_store::CharacterStore,
    character: Option<&CharacterTemplate>,
    router: &MapRouter,
    presence: &Option<super::WorldPresence>,
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

pub(crate) async fn lookup_player(
    presence: &Option<super::WorldPresence>,
    guid: u64,
) -> anyhow::Result<Option<Player>> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        super::MapBackend::Local(world) => Ok(crate::map_handle::MapHandle::player(world, guid)),
        super::MapBackend::Remote(session) => session.player(guid).await,
    }
}

async fn reply_creature(
    connection: &mut ClientConnection,
    presence: &Option<super::WorldPresence>,
    entry: u32,
    guid: u64,
) -> anyhow::Result<()> {
    let creature = lookup_creature(presence, entry, guid).await?;
    let creature = creature.filter(|creature| creature.entry == entry);
    connection.reply_creature(entry, creature.as_ref()).await
}

async fn reply_gameobject(
    connection: &mut ClientConnection,
    presence: &Option<super::WorldPresence>,
    entry: u32,
    guid: u64,
) -> anyhow::Result<()> {
    let object = lookup_gameobject(presence, entry, guid).await?;
    let object = object.filter(|object| object.entry == entry);
    connection.reply_gameobject(entry, object.as_ref()).await
}

pub(crate) async fn lookup_gameobject(
    presence: &Option<super::WorldPresence>,
    entry: u32,
    guid: u64,
) -> anyhow::Result<Option<crate::gameobject::GameObject>> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        super::MapBackend::Local(world) => {
            Ok(crate::map_handle::MapHandle::gameobject(world, guid)
                .or_else(|| crate::map_handle::MapHandle::gameobject_by_entry(world, entry)))
        }
        super::MapBackend::Remote(session) => {
            if let Some(object) = session.gameobject(guid).await? {
                return Ok(Some(object));
            }
            session.gameobject_by_entry(entry).await
        }
    }
}

pub(crate) async fn lookup_creature(
    presence: &Option<super::WorldPresence>,
    entry: u32,
    guid: u64,
) -> anyhow::Result<Option<crate::creature::Creature>> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        super::MapBackend::Local(world) => Ok(crate::map_handle::MapHandle::creature(world, guid)
            .or_else(|| crate::map_handle::MapHandle::creature_by_entry(world, entry))),
        super::MapBackend::Remote(session) => {
            if let Some(creature) = session.creature(guid).await? {
                return Ok(Some(creature));
            }
            session.creature_by_entry(entry).await
        }
    }
}

async fn lookup_item(
    presence: &Option<super::WorldPresence>,
    entry: u32,
) -> anyhow::Result<Option<crate::catalog::ItemRow>> {
    let Some(backend) = presence.as_ref().and_then(super::WorldPresence::backend) else {
        return Ok(None);
    };
    match backend {
        super::MapBackend::Local(world) => Ok(crate::map_handle::MapHandle::item(world, entry)),
        super::MapBackend::Remote(session) => session.item(entry).await,
    }
}

pub(crate) async fn lookup_player_on(
    router: &MapRouter,
    map_id: u32,
    guid: u64,
) -> anyhow::Result<Option<Player>> {
    if let Some(world) = router.map(map_id) {
        return Ok(world.player(guid));
    }
    Ok(None)
}
