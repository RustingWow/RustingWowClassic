use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub bind: SocketAddr,
    pub internal_bind: SocketAddr,
    pub world_public_addr: String,
}

#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub bind: SocketAddr,
    pub auth_internal_url: String,
    pub log_unhandled_packets: bool,
}

impl AuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind: parse_addr("AUTH_BIND", "0.0.0.0:3724")?,
            internal_bind: parse_addr("AUTH_INTERNAL_BIND", "127.0.0.1:9090")?,
            world_public_addr: env::var("WORLD_PUBLIC_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8085".to_string()),
        })
    }
}

impl WorldConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind: parse_addr("WORLD_BIND", "0.0.0.0:8085")?,
            auth_internal_url: env::var("AUTH_INTERNAL_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9090".to_string()),
            log_unhandled_packets: parse_bool("LOG_UNHANDLED_PACKETS", false)?,
        })
    }
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
