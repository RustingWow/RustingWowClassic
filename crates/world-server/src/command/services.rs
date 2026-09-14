use std::sync::{Arc, Mutex};

use crate::command::tele::TeleStore;

#[derive(Clone)]
pub struct CommandServices {
    pub tele: TeleStore,
    pub motd: Arc<Mutex<String>>,
    pub auth: Option<AuthGmClient>,
}

impl CommandServices {
    pub fn memory() -> Self {
        Self {
            tele: TeleStore::memory(),
            motd: Arc::new(Mutex::new("Welcome to WoWServer.".to_string())),
            auth: None,
        }
    }

    pub fn motd(&self) -> String {
        self.motd.lock().expect("motd mutex").clone()
    }

    pub fn set_motd(&self, text: String) {
        *self.motd.lock().expect("motd mutex") = text;
    }
}

#[derive(Clone)]
pub struct AuthGmClient {
    url: String,
    token: Option<String>,
}

impl AuthGmClient {
    pub fn new(url: impl Into<String>, token: Option<String>) -> Self {
        Self {
            url: url.into().trim_end_matches('/').to_string(),
            token,
        }
    }

    pub async fn set_gmlevel(&self, username: &str, gmlevel: u8) -> anyhow::Result<bool> {
        let mut request = reqwest::Client::new()
            .put(format!("{}/internal/accounts/gmlevel", self.url))
            .json(&serde_json::json!({ "username": username, "gmlevel": gmlevel }));
        if let Some(token) = &self.token {
            request = request.header("X-Auth-Internal-Token", token);
        }
        let response = request.send().await?;
        match response.status().as_u16() {
            204 => Ok(true),
            404 => Ok(false),
            status => anyhow::bail!("set gmlevel failed: {status}"),
        }
    }

    pub async fn list_gms(&self) -> anyhow::Result<Vec<(String, u8)>> {
        let mut request = reqwest::Client::new().get(format!("{}/internal/accounts/gm", self.url));
        if let Some(token) = &self.token {
            request = request.header("X-Auth-Internal-Token", token);
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            anyhow::bail!("list gms failed: {}", response.status());
        }
        let rows: Vec<serde_json::Value> = response.json().await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                Some((
                    row.get("username")?.as_str()?.to_string(),
                    row.get("gmlevel")?.as_u64()? as u8,
                ))
            })
            .collect())
    }
}
