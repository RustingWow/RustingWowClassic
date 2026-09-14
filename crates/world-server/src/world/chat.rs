use wow_shared::Position;

use super::{EMOTE_RANGE, Inner, PlayerMailbox, SAY_RANGE, World, WorldEvent, YELL_RANGE};

impl World {
    pub fn speak(&self, from: &PlayerMailbox, chat: Chat) {
        let inner = self.inner.lock().expect("world mutex");
        let Some(speaker) = inner.players.get(&chat.speaker) else {
            return;
        };
        if !speaker.mailbox.same_channel(from) {
            return;
        }
        let origin = speaker.player.position;
        let gm_tag = speaker.player.gm_chat;

        match chat.channel {
            ChatChannel::Say => {
                broadcast_in_range(
                    &inner,
                    origin,
                    SAY_RANGE,
                    SpokenChat {
                        from_guid: chat.speaker,
                        text: chat.text,
                        delivery: ChatDelivery::Say,
                        gm_tag,
                    },
                );
            }
            ChatChannel::Yell => {
                broadcast_in_range(
                    &inner,
                    origin,
                    YELL_RANGE,
                    SpokenChat {
                        from_guid: chat.speaker,
                        text: chat.text,
                        delivery: ChatDelivery::Yell,
                        gm_tag,
                    },
                );
            }
            ChatChannel::Emote => {
                broadcast_in_range(
                    &inner,
                    origin,
                    EMOTE_RANGE,
                    SpokenChat {
                        from_guid: chat.speaker,
                        text: chat.text,
                        delivery: ChatDelivery::Emote,
                        gm_tag,
                    },
                );
            }
            ChatChannel::Whisper { to } => {
                let target = inner
                    .players
                    .values()
                    .find(|presence| presence.player.name.eq_ignore_ascii_case(&to));
                let Some(target) = target else {
                    from.send(WorldEvent::ChatPlayerNotFound { name: to });
                    return;
                };
                target.mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: chat.speaker,
                    text: chat.text.clone(),
                    delivery: ChatDelivery::Whisper,
                    gm_tag,
                }));
                from.send(WorldEvent::Chat(SpokenChat {
                    from_guid: target.player.guid,
                    text: chat.text,
                    delivery: ChatDelivery::WhisperInform,
                    gm_tag: false,
                }));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChatChannel {
    Say,
    Yell,
    Emote,
    Whisper { to: String },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Chat {
    pub speaker: u64,
    pub channel: ChatChannel,
    pub text: String,
    #[serde(default)]
    pub gm_tag: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChatDelivery {
    Say,
    Yell,
    Emote,
    Whisper,
    WhisperInform,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpokenChat {
    pub from_guid: u64,
    pub text: String,
    pub delivery: ChatDelivery,
    #[serde(default)]
    pub gm_tag: bool,
}

fn broadcast_in_range(inner: &Inner, origin: Position, range: f32, spoken: SpokenChat) {
    let range_squared = range * range;
    let event = WorldEvent::Chat(spoken);
    for presence in inner.players.values() {
        if presence.player.position.distance_squared(origin) <= range_squared {
            presence.mailbox.send(event.clone());
        }
    }
}
