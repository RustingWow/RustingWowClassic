use tokio::net::TcpStream;
use tokio::sync::mpsc;
use wow_shared::CharacterTemplate;
use wow_srp::vanilla_header::{EncrypterHalf, HeaderCrypto};

use crate::creature::Creature;
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
        character: &CharacterTemplate,
    ) -> anyhow::Result<()> {
        packets::character_list(&mut self.writer, &mut self.encrypter, character).await
    }

    pub async fn enter_world(&mut self, character: &CharacterTemplate) -> anyhow::Result<()> {
        packets::enter_world(&mut self.writer, &mut self.encrypter, character).await
    }

    #[allow(dead_code)]
    pub async fn transfer_world(
        &mut self,
        map_id: u32,
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
            WorldEvent::GossipOpened { npc, menu } => {
                packets::gossip_opened(&mut self.writer, &mut self.encrypter, npc, menu).await
            }
            WorldEvent::GossipClosed => {
                packets::gossip_closed(&mut self.writer, &mut self.encrypter).await
            }
        }
    }

    pub async fn reply_npc_text(&mut self, text_id: u32, text: Option<&str>) -> anyhow::Result<()> {
        packets::npc_text_update(
            &mut self.writer,
            &mut self.encrypter,
            text_id,
            text.unwrap_or(""),
        )
        .await
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
