use tokio::net::TcpStream;
use tokio::sync::mpsc;
use wow_shared::{CharacterMap, CharacterTemplate};
use wow_srp::vanilla_header::{EncrypterHalf, HeaderCrypto};

use crate::creature::Creature;
use crate::gameobject::GameObject;
use crate::player::Player;
use crate::protocol::action::ClientAction;
use crate::protocol::packets;
use crate::protocol::read::{Incoming, read_incoming};
use crate::world::{Movement, PlayerMailbox, WorldEvent};

pub struct ClientConnection {
    writer: tokio::net::tcp::OwnedWriteHalf,
    encrypter: EncrypterHalf,
    incoming: mpsc::UnboundedReceiver<Incoming>,
    world_events: mpsc::UnboundedReceiver<WorldEvent>,
    mailbox: PlayerMailbox,
    reader: tokio::task::JoinHandle<()>,
}

pub enum ConnectionEvent {
    Action(ClientAction),
    World(WorldEvent),
    Disconnected,
}

impl ClientConnection {
    pub fn new(stream: TcpStream, encryption: HeaderCrypto) -> Self {
        let (read_half, writer) = stream.into_split();
        let (encrypter, mut decrypter) = encryption.split();
        let (in_tx, incoming) = mpsc::unbounded_channel();
        let (mailbox, world_events) = PlayerMailbox::channel();

        let reader = tokio::spawn(async move {
            let mut read_half = read_half;
            loop {
                match read_incoming(&mut read_half, &mut decrypter).await {
                    Ok(packet) => {
                        if in_tx.send(packet).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            writer,
            encrypter,
            incoming,
            world_events,
            mailbox,
            reader,
        }
    }

    pub fn mailbox(&self) -> PlayerMailbox {
        self.mailbox.clone()
    }

    pub async fn next(&mut self) -> ConnectionEvent {
        tokio::select! {
            incoming = self.incoming.recv() => match incoming {
                Some(incoming) => ConnectionEvent::Action(incoming.into()),
                None => ConnectionEvent::Disconnected,
            },
            event = self.world_events.recv() => match event {
                Some(event) => ConnectionEvent::World(event),
                None => ConnectionEvent::Disconnected,
            },
        }
    }

    pub async fn pong(&mut self, sequence_id: u32) -> anyhow::Result<()> {
        packets::pong(&mut self.writer, &mut self.encrypter, sequence_id).await
    }

    pub async fn send_character_list(
        &mut self,
        characters: &[CharacterTemplate],
    ) -> anyhow::Result<()> {
        packets::character_list(&mut self.writer, &mut self.encrypter, characters).await
    }

    pub async fn char_create(
        &mut self,
        result: wow_world_messages::vanilla::WorldResult,
    ) -> anyhow::Result<()> {
        packets::char_create(&mut self.writer, &mut self.encrypter, result).await
    }

    pub async fn char_delete(
        &mut self,
        result: wow_world_messages::vanilla::WorldResult,
    ) -> anyhow::Result<()> {
        packets::char_delete(&mut self.writer, &mut self.encrypter, result).await
    }

    pub async fn logout_to_character_screen(&mut self) -> anyhow::Result<()> {
        packets::logout_response(
            &mut self.writer,
            &mut self.encrypter,
            wow_world_messages::vanilla::LogoutResult::Success,
            wow_world_messages::vanilla::LogoutSpeed::Instant,
        )
        .await?;
        packets::logout_complete(&mut self.writer, &mut self.encrypter).await
    }

    pub async fn logout_cancel_ack(&mut self) -> anyhow::Result<()> {
        packets::logout_cancel_ack(&mut self.writer, &mut self.encrypter).await
    }

    pub async fn enter_world(
        &mut self,
        character: &CharacterTemplate,
        player: &Player,
    ) -> anyhow::Result<()> {
        packets::enter_world(&mut self.writer, &mut self.encrypter, character, player).await
    }

    #[allow(dead_code)]
    pub async fn transfer_world(
        &mut self,
        map_id: CharacterMap,
        position: wow_shared::Position,
    ) -> anyhow::Result<()> {
        packets::transfer_world(&mut self.writer, &mut self.encrypter, map_id, position).await
    }

    pub async fn show_players(&mut self, players: &[Player]) -> anyhow::Result<()> {
        packets::appear(&mut self.writer, &mut self.encrypter, players, false).await
    }

    pub async fn show_creatures(&mut self, creatures: &[Creature]) -> anyhow::Result<()> {
        packets::appear_creatures(&mut self.writer, &mut self.encrypter, creatures).await
    }

    pub async fn show_gameobjects(&mut self, objects: &[GameObject]) -> anyhow::Result<()> {
        packets::appear_gameobjects(&mut self.writer, &mut self.encrypter, objects).await
    }

    pub async fn reply_name(&mut self, guid: u64, player: &Player) -> anyhow::Result<()> {
        packets::name_reply(&mut self.writer, &mut self.encrypter, guid, player).await
    }

    pub async fn reply_creature(
        &mut self,
        entry: u32,
        creature: Option<&Creature>,
    ) -> anyhow::Result<()> {
        packets::creature_query(&mut self.writer, &mut self.encrypter, entry, creature).await
    }

    pub async fn reply_gameobject(
        &mut self,
        entry: u32,
        object: Option<&GameObject>,
    ) -> anyhow::Result<()> {
        packets::gameobject_query(&mut self.writer, &mut self.encrypter, entry, object).await
    }

    pub async fn apply(&mut self, event: WorldEvent) -> anyhow::Result<()> {
        match event {
            WorldEvent::PlayerAppeared(player) => {
                let players = [player];
                self.show_players(&players).await
            }
            WorldEvent::PlayerLeft { guid } => {
                packets::hide_player(&mut self.writer, &mut self.encrypter, guid).await
            }
            WorldEvent::PlayerMoved(movement) => self.replay_movement(movement).await,
            WorldEvent::Chat(spoken) => {
                packets::chat_message(&mut self.writer, &mut self.encrypter, &spoken).await
            }
            WorldEvent::ChatPlayerNotFound { name } => {
                packets::chat_player_not_found(&mut self.writer, &mut self.encrypter, name).await
            }
            WorldEvent::CreatureAppeared(creature) => {
                let creatures = [creature];
                self.show_creatures(&creatures).await
            }
            WorldEvent::CreatureLeft { guid } => {
                packets::hide_player(&mut self.writer, &mut self.encrypter, guid).await
            }
            WorldEvent::GameObjectAppeared(object) => {
                let objects = [object];
                self.show_gameobjects(&objects).await
            }
            WorldEvent::GameObjectLeft { guid } => {
                packets::hide_player(&mut self.writer, &mut self.encrypter, guid).await
            }
            WorldEvent::AttackStarted(attack) => {
                packets::attack_start(&mut self.writer, &mut self.encrypter, attack).await
            }
            WorldEvent::AttackStopped(attack) => {
                packets::attack_stop(&mut self.writer, &mut self.encrypter, attack).await
            }
            WorldEvent::MeleeHit(hit) => {
                packets::melee_hit(&mut self.writer, &mut self.encrypter, hit).await
            }
            WorldEvent::StandStateAck { state } => {
                packets::stand_state_ack(&mut self.writer, &mut self.encrypter, state).await
            }
            WorldEvent::PlayerStandState { guid, state } => {
                packets::player_stand_state(&mut self.writer, &mut self.encrypter, guid, state)
                    .await
            }
            WorldEvent::GossipOpened {
                npc,
                menu,
                quests,
                pages,
            } => {
                packets::gossip_opened(
                    &mut self.writer,
                    &mut self.encrypter,
                    npc,
                    menu,
                    quests,
                    pages,
                )
                .await
            }
            WorldEvent::GossipClosed => {
                packets::gossip_closed(&mut self.writer, &mut self.encrypter).await
            }
            WorldEvent::QuestGiverStatus { npc, status } => {
                packets::questgiver_status(&mut self.writer, &mut self.encrypter, npc, status).await
            }
            WorldEvent::QuestList { npc, title, quests } => {
                packets::quest_list(&mut self.writer, &mut self.encrypter, npc, title, quests).await
            }
            WorldEvent::QuestDetails { npc, quest } => {
                packets::quest_details(&mut self.writer, &mut self.encrypter, npc, quest).await
            }
            WorldEvent::QuestLogFull => {
                packets::quest_log_full(&mut self.writer, &mut self.encrypter).await
            }
            WorldEvent::QuestLogUpdate { player } => {
                packets::quest_log_update(&mut self.writer, &mut self.encrypter, &player).await
            }
            WorldEvent::QuestOfferReward { npc, quest } => {
                packets::quest_offer_reward(&mut self.writer, &mut self.encrypter, npc, quest).await
            }
            WorldEvent::QuestTurnedIn {
                quest_id,
                copper,
                items,
            } => {
                packets::quest_complete(
                    &mut self.writer,
                    &mut self.encrypter,
                    quest_id,
                    copper,
                    items,
                )
                .await
            }
            WorldEvent::QuestKillCredit { victim, credit } => {
                packets::quest_kill_credit(&mut self.writer, &mut self.encrypter, victim, credit)
                    .await
            }
            WorldEvent::QuestObjectivesDone { quest_id } => {
                packets::quest_objectives_done(&mut self.writer, &mut self.encrypter, quest_id)
                    .await
            }
            WorldEvent::QuestStateChanged { .. } => Ok(()),
            WorldEvent::VendorOpened { npc, items } => {
                packets::vendor_list(&mut self.writer, &mut self.encrypter, npc, items).await
            }
            WorldEvent::LootOpened { guid, gold, items } => {
                packets::loot_opened(&mut self.writer, &mut self.encrypter, guid, gold, items).await
            }
            WorldEvent::LootFailed { guid, error } => {
                packets::loot_failed(&mut self.writer, &mut self.encrypter, guid, error).await
            }
            WorldEvent::LootTaken { index } => {
                packets::loot_taken(&mut self.writer, &mut self.encrypter, index).await
            }
            WorldEvent::LootClosed { guid } => {
                packets::loot_closed(&mut self.writer, &mut self.encrypter, guid).await
            }
            WorldEvent::MoneyChanged { guid, copper } => {
                packets::money(&mut self.writer, &mut self.encrypter, guid, copper).await
            }
            WorldEvent::Notification { text } => {
                packets::notification(&mut self.writer, &mut self.encrypter, text).await
            }
            WorldEvent::ForcedTeleport { .. } => Ok(()),
            WorldEvent::CreatureMoved {
                guid,
                from,
                to,
                duration_ms,
            } => {
                packets::creature_moved(
                    &mut self.writer,
                    &mut self.encrypter,
                    guid,
                    from,
                    to,
                    duration_ms,
                )
                .await
            }
        }
    }

    pub async fn reply_item(
        &mut self,
        entry: u32,
        item: Option<&crate::catalog::ItemRow>,
    ) -> anyhow::Result<()> {
        packets::item_query(&mut self.writer, &mut self.encrypter, entry, item).await
    }

    pub async fn reply_npc_text(
        &mut self,
        text_id: u32,
        pages: &[crate::catalog::NpcTextPage; 8],
    ) -> anyhow::Result<()> {
        packets::npc_text_update(&mut self.writer, &mut self.encrypter, text_id, pages).await
    }

    pub async fn reply_quest(
        &mut self,
        quest: Option<&crate::catalog::QuestRow>,
    ) -> anyhow::Result<()> {
        let Some(quest) = quest else {
            return Ok(());
        };
        packets::quest_query(&mut self.writer, &mut self.encrypter, quest).await
    }

    async fn replay_movement(&mut self, movement: Movement) -> anyhow::Result<()> {
        let Some(packet) = movement.packet() else {
            return Ok(());
        };
        packet
            .tokio_write_encrypted_server(&mut self.writer, &mut self.encrypter)
            .await?;
        Ok(())
    }

    pub fn close(&self) {
        self.reader.abort();
    }
}
