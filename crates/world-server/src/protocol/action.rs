use wow_world_messages::vanilla::opcodes::ClientOpcodeMessage;
use wow_world_messages::vanilla::{CMSG_MESSAGECHAT, CMSG_MESSAGECHAT_ChatType};

use wow_shared::{Appearance, CharacterClass, CharacterGender, CharacterRace};

use crate::protocol::read::Incoming;
use crate::world::{Chat, ChatChannel, Movement};

pub enum ClientAction {
    Ping {
        sequence_id: u32,
    },
    ListCharacters,
    CreateCharacter {
        name: String,
        race: CharacterRace,
        class: CharacterClass,
        gender: CharacterGender,
        appearance: Appearance,
    },
    CreateCharacterRejected,
    DeleteCharacter {
        guid: u64,
    },
    EnterWorld {
        guid: u64,
    },
    QueryName {
        guid: u64,
    },
    QueryCreature {
        entry: u32,
        guid: u64,
    },
    Select,
    Attack {
        guid: u64,
    },
    StopAttack,
    ChangeStandState {
        state: u8,
    },
    GossipHello {
        guid: u64,
    },
    GossipSelect {
        guid: u64,
        option: u32,
    },
    QueryNpcText {
        text_id: u32,
    },
    LogoutRequest,
    PlayerLogout,
    LogoutCancel,
    Moved(PendingMove),
    Chat(PendingChat),
    Ignored(IgnoredAction),
}

pub struct IgnoredAction {
    pub opcode: Option<u32>,
    pub name: Option<String>,
}

pub struct PendingMove {
    opcode: ClientOpcodeMessage,
}

impl PendingMove {
    pub fn bind(self, guid: u64) -> Option<Movement> {
        crate::protocol::movement::from_client(&self.opcode, guid)
    }
}

pub struct PendingChat {
    channel: ChatChannel,
    text: String,
}

impl PendingChat {
    pub fn bind(self, speaker: u64) -> Chat {
        Chat {
            speaker,
            channel: self.channel,
            text: self.text,
        }
    }
}

impl From<Incoming> for ClientAction {
    fn from(incoming: Incoming) -> Self {
        match incoming {
            Incoming::Skipped { opcode, name } => Self::Ignored(IgnoredAction {
                opcode: Some(opcode),
                name: name.map(str::to_string),
            }),
            Incoming::Message(message) => match message {
                ClientOpcodeMessage::CMSG_PING(ping) => Self::Ping {
                    sequence_id: ping.sequence_id,
                },
                ClientOpcodeMessage::CMSG_CHAR_ENUM => Self::ListCharacters,
                ClientOpcodeMessage::CMSG_CHAR_CREATE(create) => {
                    match (
                        CharacterRace::from_protocol(create.race.as_int()),
                        CharacterClass::from_protocol(create.class.as_int()),
                        CharacterGender::from_protocol(create.gender.as_int()),
                    ) {
                        (Some(race), Some(class), Some(gender)) => Self::CreateCharacter {
                            name: create.name.clone(),
                            race,
                            class,
                            gender,
                            appearance: Appearance {
                                skin: create.skin_color,
                                face: create.face,
                                hair_style: create.hair_style,
                                hair_color: create.hair_color,
                                facial_hair: create.facial_hair,
                            },
                        },
                        _ => Self::CreateCharacterRejected,
                    }
                }
                ClientOpcodeMessage::CMSG_CHAR_DELETE(delete) => Self::DeleteCharacter {
                    guid: delete.guid.guid(),
                },
                ClientOpcodeMessage::CMSG_PLAYER_LOGIN(login) => Self::EnterWorld {
                    guid: login.guid.guid(),
                },
                ClientOpcodeMessage::CMSG_NAME_QUERY(query) => Self::QueryName {
                    guid: query.guid.guid(),
                },
                ClientOpcodeMessage::CMSG_CREATURE_QUERY(query) => Self::QueryCreature {
                    entry: query.creature,
                    guid: query.guid.guid(),
                },
                ClientOpcodeMessage::CMSG_SET_SELECTION(_) => Self::Select,
                ClientOpcodeMessage::CMSG_ATTACKSWING(swing) => Self::Attack {
                    guid: swing.guid.guid(),
                },
                ClientOpcodeMessage::CMSG_ATTACKSTOP => Self::StopAttack,
                ClientOpcodeMessage::CMSG_STANDSTATECHANGE(change) => Self::ChangeStandState {
                    state: change.animation_state.as_int(),
                },
                ClientOpcodeMessage::CMSG_GOSSIP_HELLO(hello) => Self::GossipHello {
                    guid: hello.guid.guid(),
                },
                ClientOpcodeMessage::CMSG_GOSSIP_SELECT_OPTION(select) => Self::GossipSelect {
                    guid: select.guid.guid(),
                    option: select.gossip_list_id,
                },
                ClientOpcodeMessage::CMSG_NPC_TEXT_QUERY(query) => Self::QueryNpcText {
                    text_id: query.text_id,
                },
                ClientOpcodeMessage::CMSG_LOGOUT_REQUEST => Self::LogoutRequest,
                ClientOpcodeMessage::CMSG_PLAYER_LOGOUT => Self::PlayerLogout,
                ClientOpcodeMessage::CMSG_LOGOUT_CANCEL => Self::LogoutCancel,
                ClientOpcodeMessage::CMSG_MESSAGECHAT(message) => {
                    match chat_from_client(*message) {
                        Some(chat) => Self::Chat(chat),
                        None => Self::Ignored(IgnoredAction {
                            opcode: None,
                            name: Some("CMSG_MESSAGECHAT".to_string()),
                        }),
                    }
                }
                other => {
                    if other.movement_info().is_some() {
                        Self::Moved(PendingMove { opcode: other })
                    } else {
                        Self::Ignored(IgnoredAction {
                            opcode: None,
                            name: Some(other.to_string()),
                        })
                    }
                }
            },
        }
    }
}

fn chat_from_client(message: CMSG_MESSAGECHAT) -> Option<PendingChat> {
    if message.message.is_empty() {
        return None;
    }
    let channel = match message.chat_type {
        CMSG_MESSAGECHAT_ChatType::Say => ChatChannel::Say,
        CMSG_MESSAGECHAT_ChatType::Yell => ChatChannel::Yell,
        CMSG_MESSAGECHAT_ChatType::Emote => ChatChannel::Emote,
        CMSG_MESSAGECHAT_ChatType::Whisper { target_player } => {
            ChatChannel::Whisper { to: target_player }
        }
        _ => return None,
    };
    Some(PendingChat {
        channel,
        text: message.message,
    })
}
