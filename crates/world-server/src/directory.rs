use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::world::{ChatDelivery, PlayerMailbox, SpokenChat, WorldEvent};

#[derive(Clone, Debug)]
pub struct OnlinePlayer {
    pub guid: u64,
    pub name: String,
    pub map_id: u32,
    pub gmlevel: u8,
}

#[derive(Clone)]
pub struct Directory {
    inner: Arc<Mutex<Inner>>,
    redis: Option<String>,
}

struct Inner {
    by_guid: HashMap<u64, Entry>,
    by_name: HashMap<String, u64>,
}

struct Entry {
    guid: u64,
    name: String,
    map_id: u32,
    gmlevel: u8,
    mailbox: PlayerMailbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WhisperWire {
    target_guid: u64,
    from_guid: u64,
    text: String,
    inform: bool,
}

impl Directory {
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                by_guid: HashMap::new(),
                by_name: HashMap::new(),
            })),
            redis: None,
        }
    }

    pub fn with_redis(url: impl Into<String>) -> Self {
        let url = url.into();
        let dir = Self {
            inner: Arc::new(Mutex::new(Inner {
                by_guid: HashMap::new(),
                by_name: HashMap::new(),
            })),
            redis: Some(url.clone()),
        };
        dir.spawn_whisper_subscriber();
        dir
    }

    pub fn register(&self, guid: u64, name: String, map_id: u32, mailbox: PlayerMailbox) {
        let key = name.to_ascii_lowercase();
        {
            let mut inner = self.inner.lock().expect("directory mutex");
            if let Some(old) = inner.by_guid.remove(&guid) {
                inner.by_name.remove(&old.name.to_ascii_lowercase());
            }
            inner.by_name.insert(key.clone(), guid);
            inner.by_guid.insert(
                guid,
                Entry {
                    guid,
                    name: name.clone(),
                    map_id,
                    gmlevel: 0,
                    mailbox,
                },
            );
        }
        self.redis_register(guid, &name, map_id);
    }

    pub fn set_map(&self, guid: u64, map_id: u32) {
        if let Some(entry) = self
            .inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get_mut(&guid)
        {
            entry.map_id = map_id;
        }
        self.redis_set_map(guid, map_id);
    }

    pub fn unregister(&self, guid: u64, mailbox: &PlayerMailbox) {
        let name = {
            let mut inner = self.inner.lock().expect("directory mutex");
            let Some(entry) = inner.by_guid.get(&guid) else {
                return;
            };
            if !entry.mailbox.same_channel(mailbox) {
                return;
            }
            let entry = inner.by_guid.remove(&guid).expect("present");
            inner.by_name.remove(&entry.name.to_ascii_lowercase());
            entry.name
        };
        self.redis_unregister(guid, &name);
    }

    pub fn name(&self, guid: u64) -> Option<String> {
        self.inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get(&guid)
            .map(|entry| entry.name.clone())
            .or_else(|| self.redis_name(guid))
    }

    pub fn guid_by_name(&self, name: &str) -> Option<u64> {
        let key = name.to_ascii_lowercase();
        self.inner
            .lock()
            .expect("directory mutex")
            .by_name
            .get(&key)
            .copied()
            .or_else(|| self.redis_guid(&key))
    }

    pub fn map_of(&self, guid: u64) -> Option<u32> {
        self.inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get(&guid)
            .map(|entry| entry.map_id)
            .or_else(|| self.redis_map(guid))
    }

    pub fn who(&self) -> Vec<(String, u32)> {
        self.online()
            .into_iter()
            .map(|player| (player.name, player.map_id))
            .collect()
    }

    pub fn online(&self) -> Vec<OnlinePlayer> {
        let local: Vec<_> = self
            .inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .values()
            .map(|entry| OnlinePlayer {
                guid: entry.guid,
                name: entry.name.clone(),
                map_id: entry.map_id,
                gmlevel: entry.gmlevel,
            })
            .collect();
        if !local.is_empty() || self.redis.is_none() {
            return local;
        }
        self.redis_who()
            .unwrap_or_default()
            .into_iter()
            .map(|(name, map_id)| OnlinePlayer {
                guid: 0,
                name,
                map_id,
                gmlevel: 0,
            })
            .collect()
    }

    pub fn set_gmlevel(&self, guid: u64, gmlevel: u8) {
        if let Some(entry) = self
            .inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get_mut(&guid)
        {
            entry.gmlevel = gmlevel;
        }
    }

    pub fn mailbox(&self, guid: u64) -> Option<PlayerMailbox> {
        self.inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get(&guid)
            .map(|entry| entry.mailbox.clone())
    }

    pub fn player_at(&self, guid: u64) -> Option<(String, u32)> {
        self.inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get(&guid)
            .map(|entry| (entry.name.clone(), entry.map_id))
    }

    pub fn whisper(&self, from: &PlayerMailbox, speaker: u64, to: String, text: String) {
        let target_guid = {
            let inner = self.inner.lock().expect("directory mutex");
            let Some(speaker_entry) = inner.by_guid.get(&speaker) else {
                return;
            };
            if !speaker_entry.mailbox.same_channel(from) {
                return;
            }
            inner.by_name.get(&to.to_ascii_lowercase()).copied()
        };
        let Some(target_guid) = target_guid.or_else(|| self.redis_guid(&to.to_ascii_lowercase()))
        else {
            from.send(WorldEvent::ChatPlayerNotFound { name: to });
            return;
        };

        let delivered = {
            let inner = self.inner.lock().expect("directory mutex");
            if let Some(target) = inner.by_guid.get(&target_guid) {
                target.mailbox.send(WorldEvent::Chat(SpokenChat {
                    from_guid: speaker,
                    text: text.clone(),
                    delivery: ChatDelivery::Whisper,
                    gm_tag: false,
                }));
                from.send(WorldEvent::Chat(SpokenChat {
                    from_guid: target.guid,
                    text: text.clone(),
                    delivery: ChatDelivery::WhisperInform,
                    gm_tag: false,
                }));
                true
            } else {
                false
            }
        };
        if delivered {
            return;
        }
        from.send(WorldEvent::Chat(SpokenChat {
            from_guid: target_guid,
            text: text.clone(),
            delivery: ChatDelivery::WhisperInform,
            gm_tag: false,
        }));
        self.redis_publish_whisper(target_guid, speaker, text);
    }

    fn redis_register(&self, guid: u64, name: &str, map_id: u32) {
        let Some(url) = &self.redis else {
            return;
        };
        if let Ok(mut conn) = redis_conn(url) {
            let _: redis::RedisResult<()> = redis::pipe()
                .set(player_key(guid), format!("{name}\n{map_id}"))
                .set(name_key(name), guid)
                .sadd("wow:online", guid)
                .query(&mut conn);
        }
    }

    fn redis_set_map(&self, guid: u64, map_id: u32) {
        let Some(url) = &self.redis else {
            return;
        };
        let Some(name) = self
            .inner
            .lock()
            .expect("directory mutex")
            .by_guid
            .get(&guid)
            .map(|entry| entry.name.clone())
        else {
            return;
        };
        if let Ok(mut conn) = redis_conn(url) {
            let _: redis::RedisResult<()> = redis::cmd("SET")
                .arg(player_key(guid))
                .arg(format!("{name}\n{map_id}"))
                .query(&mut conn);
        }
    }

    fn redis_unregister(&self, guid: u64, name: &str) {
        let Some(url) = &self.redis else {
            return;
        };
        if let Ok(mut conn) = redis_conn(url) {
            let _: redis::RedisResult<()> = redis::pipe()
                .del(player_key(guid))
                .del(name_key(name))
                .srem("wow:online", guid)
                .query(&mut conn);
        }
    }

    fn redis_name(&self, guid: u64) -> Option<String> {
        let (name, _) = self.redis_player(guid)?;
        Some(name)
    }

    fn redis_map(&self, guid: u64) -> Option<u32> {
        let (_, map_id) = self.redis_player(guid)?;
        Some(map_id)
    }

    fn redis_player(&self, guid: u64) -> Option<(String, u32)> {
        let url = self.redis.as_ref()?;
        let mut conn = redis_conn(url).ok()?;
        let value: String = redis::cmd("GET")
            .arg(player_key(guid))
            .query(&mut conn)
            .ok()?;
        let (name, map) = value.split_once('\n')?;
        Some((name.to_string(), map.parse().ok()?))
    }

    fn redis_guid(&self, name_lower: &str) -> Option<u64> {
        let url = self.redis.as_ref()?;
        let mut conn = redis_conn(url).ok()?;
        redis::cmd("GET")
            .arg(format!("wow:name:{name_lower}"))
            .query(&mut conn)
            .ok()
    }

    fn redis_who(&self) -> Option<Vec<(String, u32)>> {
        let url = self.redis.as_ref()?;
        let mut conn = redis_conn(url).ok()?;
        let ids: Vec<u64> = redis::cmd("SMEMBERS")
            .arg("wow:online")
            .query(&mut conn)
            .ok()?;
        Some(
            ids.into_iter()
                .filter_map(|guid| self.redis_player(guid))
                .collect(),
        )
    }

    fn redis_publish_whisper(&self, target_guid: u64, from_guid: u64, text: String) {
        let Some(url) = &self.redis else {
            return;
        };
        let Ok(payload) = serde_json::to_string(&WhisperWire {
            target_guid,
            from_guid,
            text,
            inform: false,
        }) else {
            return;
        };
        if let Ok(mut conn) = redis_conn(url) {
            let _: redis::RedisResult<()> = redis::cmd("PUBLISH")
                .arg("wow:whisper")
                .arg(payload)
                .query(&mut conn);
        }
    }

    fn spawn_whisper_subscriber(&self) {
        let Some(url) = self.redis.clone() else {
            return;
        };
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("whisper-sub".into())
            .spawn(move || {
                let Ok(client) = redis::Client::open(url.as_str()) else {
                    return;
                };
                let Ok(mut conn) = client.get_connection() else {
                    tracing::warn!("redis whisper subscribe failed");
                    return;
                };
                let mut pubsub = conn.as_pubsub();
                if pubsub.subscribe("wow:whisper").is_err() {
                    tracing::warn!("redis whisper subscribe failed");
                    return;
                };
                loop {
                    let Ok(msg) = pubsub.get_message() else {
                        break;
                    };
                    let Ok(payload) = msg.get_payload::<String>() else {
                        continue;
                    };
                    let Ok(wire) = serde_json::from_str::<WhisperWire>(&payload) else {
                        continue;
                    };
                    let inner = inner.lock().expect("directory mutex");
                    if let Some(target) = inner.by_guid.get(&wire.target_guid) {
                        target.mailbox.send(WorldEvent::Chat(SpokenChat {
                            from_guid: wire.from_guid,
                            text: wire.text,
                            delivery: ChatDelivery::Whisper,
                            gm_tag: false,
                        }));
                    }
                }
            })
            .ok();
    }
}

fn redis_conn(url: &str) -> redis::RedisResult<redis::Connection> {
    redis::Client::open(url)?.get_connection()
}

fn player_key(guid: u64) -> String {
    format!("wow:player:{guid}")
}

fn name_key(name: &str) -> String {
    format!("wow:name:{}", name.to_ascii_lowercase())
}

#[cfg(test)]
#[path = "../test/directory.rs"]
mod tests;
