use wow_shared::{Account, CharacterTemplate};

use crate::command::{CommandCtx, CommandServices, CreatureInfo, Effect, OnlineInfo, dispatch};
use crate::player::Player;
use crate::protocol::ClientConnection;
use crate::router::MapRouter;
use crate::world::{ChatDelivery, PlayerMailbox, SpokenChat, WorldEvent};

use super::character;
use super::query::{lookup_creature, lookup_player, lookup_player_on};
use super::{GmSession, MapBackend, WorldPresence};

pub(super) async fn execute_command(
    text: String,
    connection: &mut ClientConnection,
    account: &Account,
    store: &crate::character_store::CharacterStore,
    character: &mut CharacterTemplate,
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    services: &CommandServices,
    gm: &mut GmSession,
) -> anyhow::Result<bool> {
    let Some(presence_ref) = presence.as_ref() else {
        return Ok(true);
    };
    let catalog = match presence_ref.backend() {
        Some(MapBackend::Local(world)) => world.catalog(),
        _ => None,
    };
    let player = lookup_player(presence, character.guid)
        .await?
        .unwrap_or_else(|| Player::from(&*character));
    let selected_creature = match gm.selection {
        Some(guid) => lookup_creature(presence, 0, guid).await?,
        None => None,
    };
    let selected_player_name = match gm.selection {
        Some(guid) => router.directory().name(guid),
        None => None,
    };
    let online: Vec<OnlineInfo> = router
        .directory()
        .online()
        .into_iter()
        .map(|player| OnlineInfo {
            name: player.name,
            map_id: player.map_id,
            gmlevel: player.gmlevel,
        })
        .collect();
    let motd = services.motd();
    let ctx = CommandCtx {
        account,
        character_name: &character.name,
        character_guid: character.guid,
        map_id: presence_ref.map_id(),
        position: character.position,
        copper: player.copper,
        health: player.health,
        max_health: player.max_health,
        selection: gm.selection,
        selected_creature: selected_creature.map(|creature| CreatureInfo {
            guid: creature.guid,
            entry: creature.entry,
            name: creature.name,
            position: creature.position,
            health: creature.health,
            max_health: creature.max_health,
            level: creature.level,
            faction: creature.faction,
            hostile: creature.hostile,
        }),
        selected_player_name,
        gm_on: gm.on,
        gm_visible: gm.visible,
        gm_chat: gm.chat,
        catalog: catalog.as_deref(),
        tele: &services.tele,
        online: &online,
        motd: &motd,
        player_count: online.len(),
    };
    let result = dispatch(&text, &ctx);
    for message in result.messages {
        mailbox.send(WorldEvent::Chat(SpokenChat {
            from_guid: 0,
            text: message,
            delivery: ChatDelivery::System,
            gm_tag: false,
        }));
    }
    let mut stay = true;
    for effect in result.effects {
        if !apply_effect(
            effect, connection, account, store, character, router, mailbox, presence, services, gm,
        )
        .await?
        {
            stay = false;
        }
    }
    Ok(stay)
}

async fn apply_effect(
    effect: Effect,
    connection: &mut ClientConnection,
    account: &Account,
    store: &crate::character_store::CharacterStore,
    character: &mut CharacterTemplate,
    router: &MapRouter,
    mailbox: &PlayerMailbox,
    presence: &mut Option<WorldPresence>,
    services: &CommandServices,
    gm: &mut GmSession,
) -> anyhow::Result<bool> {
    match effect {
        Effect::Teleport { map_id, position } => {
            character::teleport_in_world(
                connection, store, character, router, mailbox, presence, account, gm, map_id,
                position,
            )
            .await?;
        }
        Effect::Recall => {
            let Some((map_id, position)) = gm.recall else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: "No previous location.".into(),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
                return Ok(true);
            };
            character::teleport_in_world(
                connection, store, character, router, mailbox, presence, account, gm, map_id,
                position,
            )
            .await?;
        }
        Effect::Appear { name } => {
            let Some(guid) = router.directory().guid_by_name(&name) else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Player '{name}' is not online."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
                return Ok(true);
            };
            let Some(map_id) = router.directory().map_of(guid) else {
                return Ok(true);
            };
            let Some(target) = lookup_player_on(router, map_id, guid).await? else {
                return Ok(true);
            };
            character::teleport_in_world(
                connection,
                store,
                character,
                router,
                mailbox,
                presence,
                account,
                gm,
                map_id,
                target.position,
            )
            .await?;
        }
        Effect::Summon { name } => {
            let Some(guid) = router.directory().guid_by_name(&name) else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Player '{name}' is not online."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
                return Ok(true);
            };
            if let Some(target_box) = router.directory().mailbox(guid) {
                target_box.send(WorldEvent::ForcedTeleport {
                    map_id: presence.as_ref().map(WorldPresence::map_id).unwrap_or(0),
                    position: character.position,
                });
            }
        }
        Effect::Kick => {
            character::leave_world(connection, store, &mut Some(character.clone()), presence)
                .await?;
            return Ok(false);
        }
        Effect::Announce {
            text,
            named,
            gm_only,
            notify,
        } => {
            let message = if named {
                format!("[{}]: {text}", character.name)
            } else if gm_only {
                format!("|cffffcc00[GM]|r {text}")
            } else {
                format!("SERVER: {text}")
            };
            for player in router.directory().online() {
                if gm_only && player.gmlevel == 0 {
                    continue;
                }
                let Some(target) = router.directory().mailbox(player.guid) else {
                    continue;
                };
                if notify {
                    target.send(WorldEvent::Notification {
                        text: message.clone(),
                    });
                } else {
                    target.send(WorldEvent::Chat(SpokenChat {
                        from_guid: 0,
                        text: message.clone(),
                        delivery: ChatDelivery::System,
                        gm_tag: false,
                    }));
                }
            }
        }
        Effect::SetGmOn(on) => {
            gm.on = on;
            if on {
                gm.chat = true;
            }
            set_gm_flags(
                presence,
                mailbox,
                character.guid,
                Some(on),
                None,
                Some(gm.chat),
            )
            .await?;
        }
        Effect::SetGmVisible(visible) => {
            gm.visible = visible;
            set_gm_flags(presence, mailbox, character.guid, None, Some(visible), None).await?;
        }
        Effect::SetGmChat(chat) => {
            gm.chat = chat;
            set_gm_flags(presence, mailbox, character.guid, None, None, Some(chat)).await?;
        }
        Effect::ModifyMoney(delta) => {
            let current = lookup_player(presence, character.guid)
                .await?
                .map(|player| player.copper)
                .unwrap_or(0);
            let copper = if delta >= 0 {
                current.saturating_add(delta as u32)
            } else {
                current.saturating_sub(delta.unsigned_abs() as u32)
            };
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                world.set_money(mailbox, character.guid, copper);
            }
        }
        Effect::SetHp { health } => {
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                world.set_health(character.guid, health, false);
            }
        }
        Effect::Die { guid } => {
            let guid = guid.unwrap_or(character.guid);
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                world.set_health(guid, 0, true);
            }
        }
        Effect::Revive { guid } => {
            let guid = guid.unwrap_or(character.guid);
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                let max = lookup_player(presence, guid)
                    .await?
                    .map(|player| player.max_health)
                    .unwrap_or(crate::player::PLAYER_MAX_HEALTH);
                world.set_health(guid, max, false);
            }
        }
        Effect::AddItem { entry, count } => {
            let stackable = match presence.as_ref().and_then(WorldPresence::backend) {
                Some(MapBackend::Local(world)) => world
                    .item(entry)
                    .map(|item| item.stackable.max(1))
                    .unwrap_or(1),
                _ => 1,
            };
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                world.add_item_to_player(character.guid, entry, count, stackable);
            }
        }
        Effect::SpawnNpc { entry } => {
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                world.spawn_temp_npc(entry, character.position);
            }
        }
        Effect::DeleteNpc { guid } => {
            if let Some(MapBackend::Local(world)) =
                presence.as_ref().and_then(WorldPresence::backend)
            {
                world.delete_npc(guid);
            }
        }
        Effect::Save => {
            character::persist_position(store, Some(character), presence.as_ref()).await?;
        }
        Effect::SetGmLevel { username, level } => {
            let Some(auth) = &services.auth else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: "Account service is unavailable.".into(),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
                return Ok(true);
            };
            match auth.set_gmlevel(&username, level).await {
                Ok(true) => mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("{username} gmlevel set to {level}. Re-login required."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                })),
                Ok(false) => mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Unknown account {username}."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                })),
                Err(error) => mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Failed to set gmlevel: {error}"),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                })),
            }
        }
        Effect::TeleAdd { name } => {
            let map_id = presence.as_ref().map(WorldPresence::map_id).unwrap_or(0);
            if services.tele.add(&name, map_id, character.position).await? {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Saved teleport '{name}'."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
            } else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Teleport '{name}' already exists."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
            }
        }
        Effect::TeleDel { name } => {
            if services.tele.delete(&name).await? {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Deleted teleport '{name}'."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
            } else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Unknown teleport '{name}'."),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
            }
        }
        Effect::SetMotd(text) => services.set_motd(text),
        Effect::GmList => {
            let Some(auth) = &services.auth else {
                mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: "Account service is unavailable.".into(),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                }));
                return Ok(true);
            };
            match auth.list_gms().await {
                Ok(gms) if gms.is_empty() => mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: "No GM accounts.".into(),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                })),
                Ok(gms) => {
                    for (username, level) in gms {
                        mailbox.send(WorldEvent::Chat(SpokenChat {
                            from_guid: 0,
                            text: format!("{username} (gm{level})"),
                            delivery: ChatDelivery::System,
                            gm_tag: false,
                        }));
                    }
                }
                Err(error) => mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: 0,
                    text: format!("Failed to list GMs: {error}"),
                    delivery: ChatDelivery::System,
                    gm_tag: false,
                })),
            }
        }
    }
    Ok(true)
}

async fn set_gm_flags(
    presence: &Option<WorldPresence>,
    _mailbox: &PlayerMailbox,
    guid: u64,
    on: Option<bool>,
    visible: Option<bool>,
    chat: Option<bool>,
) -> anyhow::Result<()> {
    if let Some(MapBackend::Local(world)) = presence.as_ref().and_then(WorldPresence::backend) {
        world.set_gm_flags(guid, on, visible, chat);
    }
    Ok(())
}
