use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::shards::ShardFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldRole {
    Combined,
    Gateway,
    Map,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub bind: SocketAddr,
    pub internal_bind: SocketAddr,
    pub world_public_addr: String,
    pub redis_url: Option<String>,
    pub database_url: String,
    pub internal_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub bind: SocketAddr,
    pub auth_internal_url: String,
    pub log_unhandled_packets: bool,
    pub shards: ShardFile,
    pub role: WorldRole,
    pub map_ids: Vec<u32>,
    pub map_bind: SocketAddr,
    pub map_endpoints: HashMap<u32, SocketAddr>,
    pub redis_url: Option<String>,
    pub database_url: String,
    pub auth_internal_token: Option<String>,
    pub dbc_dir: PathBuf,
}

impl AuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind: parse_addr("AUTH_BIND", "0.0.0.0:3724")?,
            internal_bind: parse_addr("AUTH_INTERNAL_BIND", "127.0.0.1:9090")?,
            world_public_addr: env::var("WORLD_PUBLIC_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8085".to_string()),
            redis_url: optional_env("REDIS_URL"),
            database_url: required_env("DATABASE_URL")?,
            internal_token: optional_env("AUTH_INTERNAL_TOKEN"),
        })
    }
}

impl WorldConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let shards_path = env::var("SHARDS_FILE").unwrap_or_else(|_| "shards.yaml".to_string());
        let shards = ShardFile::load_or_default(&PathBuf::from(shards_path));
        let role = parse_role(&env::var("WORLD_ROLE").unwrap_or_else(|_| "combined".to_string()))?;
        let map_ids = parse_map_ids("MAP_IDS", &shards)?;
        Ok(Self {
            bind: parse_addr("WORLD_BIND", "0.0.0.0:8085")?,
            auth_internal_url: env::var("AUTH_INTERNAL_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9090".to_string()),
            log_unhandled_packets: parse_bool("LOG_UNHANDLED_PACKETS", false)?,
            map_bind: parse_addr("MAP_BIND", "127.0.0.1:9100")?,
            map_endpoints: parse_map_endpoints("MAP_ENDPOINTS")?,
            redis_url: optional_env("REDIS_URL"),
            database_url: required_env("DATABASE_URL")?,
            auth_internal_token: optional_env("AUTH_INTERNAL_TOKEN"),
            dbc_dir: env::var("DBC_DIR")
                .ok()
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data/dbc")),
            shards,
            role,
            map_ids,
        })
    }

    pub fn hosted_maps(&self) -> Vec<u32> {
        if self.map_ids.is_empty() {
            self.shards.maps()
        } else {
            self.map_ids.clone()
        }
    }
}

fn required_env(var: &str) -> anyhow::Result<String> {
    env::var(var).map_err(|_| anyhow::anyhow!("{var} is required"))
}

fn optional_env(var: &str) -> Option<String> {
    match env::var(var) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

fn parse_role(value: &str) -> anyhow::Result<WorldRole> {
    match value.trim().to_ascii_lowercase().as_str() {
        "combined" => Ok(WorldRole::Combined),
        "gateway" => Ok(WorldRole::Gateway),
        "map" => Ok(WorldRole::Map),
        other => anyhow::bail!("WORLD_ROLE={other} is invalid (combined|gateway|map)"),
    }
}

fn parse_map_ids(var: &str, shards: &ShardFile) -> anyhow::Result<Vec<u32>> {
    let Ok(value) = env::var(var) else {
        return Ok(Vec::new());
    };
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for part in value.split(',') {
        let id: u32 = part.trim().parse().map_err(|_| {
            anyhow::anyhow!("{var}={value} is not a comma-separated list of map ids")
        })?;
        if shards.shard_for_map(id).is_none() && !shards.maps().is_empty() {
            tracing::warn!(
                map_id = id,
                "MAP_IDS includes a map not listed in shards.yaml"
            );
        }
        ids.push(id);
    }
    Ok(ids)
}

fn parse_map_endpoints(var: &str) -> anyhow::Result<HashMap<u32, SocketAddr>> {
    let Ok(value) = env::var(var) else {
        return Ok(HashMap::new());
    };
    if value.trim().is_empty() {
        return Ok(HashMap::new());
    }
    let mut endpoints = HashMap::new();
    for part in value.split(',') {
        let Some((map, addr)) = part.split_once('=') else {
            anyhow::bail!("{var} entries must look like 0=127.0.0.1:9100");
        };
        let map_id: u32 = map
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("{var}: '{map}' is not a map id"))?;
        let addr: SocketAddr = addr
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("{var}: {addr} is not a socket address: {e}"))?;
        endpoints.insert(map_id, addr);
    }
    Ok(endpoints)
}

fn parse_addr(var: &str, default: &str) -> anyhow::Result<SocketAddr> {
    let value = env::var(var).unwrap_or_else(|_| default.to_string());
    value
        .parse()
        .map_err(|e| anyhow::anyhow!("{var}={value} is not a valid socket address: {e}"))
}

fn parse_bool(var: &str, default: bool) -> anyhow::Result<bool> {
    let Ok(value) = env::var(var) else {
        return Ok(default);
    };
    parse_bool_value(&value).ok_or_else(|| {
        anyhow::anyhow!("{var}={value} is not a boolean (use 1/0, true/false, yes/no, on/off)")
    })
}

fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../test/config.rs"]
mod tests;
