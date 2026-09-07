use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use wow_shared::SessionInfo;

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub info: SessionInfo,
    pub _created_at: Instant,
}

#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<DashMap<String, StoredSession>>,
    redis_url: Option<String>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::memory()
    }
}

impl SessionStore {
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            redis_url: None,
        }
    }

    pub fn new(redis_url: Option<String>) -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            redis_url,
        }
    }

    pub fn insert(&self, info: SessionInfo) {
        let account = info.account.clone();
        if let Some(url) = &self.redis_url {
            if let Ok(mut conn) = redis_conn(url) {
                if let Ok(payload) = serde_json::to_vec(&info) {
                    let _: redis::RedisResult<()> = redis::cmd("SET")
                        .arg(session_key(&account))
                        .arg(payload)
                        .query(&mut conn);
                }
            }
        }
        self.inner.insert(
            account,
            StoredSession {
                info,
                _created_at: Instant::now(),
            },
        );
    }

    pub fn get(&self, account: &str) -> Option<SessionInfo> {
        let upper = account.to_ascii_uppercase();
        if let Some(url) = &self.redis_url {
            if let Ok(mut conn) = redis_conn(url) {
                let blob: redis::RedisResult<Vec<u8>> =
                    redis::cmd("GET").arg(session_key(&upper)).query(&mut conn);
                if let Ok(blob) = blob {
                    if let Ok(info) = serde_json::from_slice::<SessionInfo>(&blob) {
                        return Some(info);
                    }
                }
            }
        }
        self.inner.get(&upper).map(|s| s.info.clone())
    }
}

fn session_key(account: &str) -> String {
    format!("wow:session:{}", account.to_ascii_uppercase())
}

fn redis_conn(url: &str) -> redis::RedisResult<redis::Connection> {
    redis::Client::open(url)?.get_connection()
}
