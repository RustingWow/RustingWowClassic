use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use wow_shared::SessionInfo;

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub info: SessionInfo,
    pub _created_at: Instant,
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<DashMap<String, StoredSession>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, info: SessionInfo) {
        let account = info.account.clone();
        self.inner.insert(
            account,
            StoredSession {
                info,
                _created_at: Instant::now(),
            },
        );
    }

    pub fn get(&self, account: &str) -> Option<SessionInfo> {
        self.inner
            .get(&account.to_ascii_uppercase())
            .map(|s| s.info.clone())
    }
}
