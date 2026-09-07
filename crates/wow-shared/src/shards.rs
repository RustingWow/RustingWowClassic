use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

use crate::map::{MAP_EASTERN_KINGDOMS, MAP_KALIMDOR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardKind {
    Continent,
    Instance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardSpec {
    pub name: String,
    pub maps: Vec<u32>,
    pub dedicated: bool,
    pub kind: ShardKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardFile {
    pub shards: Vec<ShardSpec>,
}

impl ShardFile {
    pub fn builtin() -> Self {
        Self {
            shards: vec![
                ShardSpec {
                    name: "eastern-kingdoms".to_string(),
                    maps: vec![MAP_EASTERN_KINGDOMS],
                    dedicated: true,
                    kind: ShardKind::Continent,
                },
                ShardSpec {
                    name: "kalimdor".to_string(),
                    maps: vec![MAP_KALIMDOR],
                    dedicated: true,
                    kind: ShardKind::Continent,
                },
            ],
        }
    }

    pub fn parse(yaml: &str) -> anyhow::Result<Self> {
        let raw: RawFile = serde_yaml::from_str(yaml)?;
        let mut seen = HashSet::new();
        let mut shards = Vec::with_capacity(raw.shards.len());
        for shard in raw.shards {
            if shard.maps.is_empty() {
                anyhow::bail!("shard '{}' has no maps", shard.name);
            }
            for map_id in &shard.maps {
                if !seen.insert(*map_id) {
                    anyhow::bail!("map {map_id} is listed in more than one shard");
                }
            }
            let kind = match shard.kind.as_deref() {
                None | Some("continent") => ShardKind::Continent,
                Some("instance") => ShardKind::Instance,
                Some(other) => anyhow::bail!("unknown shard kind '{other}'"),
            };
            shards.push(ShardSpec {
                name: shard.name,
                maps: shard.maps,
                dedicated: shard.dedicated,
                kind,
            });
        }
        Ok(Self { shards })
    }

    pub fn load_or_default(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match Self::parse(&contents) {
                Ok(file) => file,
                Err(error) => {
                    tracing::warn!(
                        path = %path.display(),
                        %error,
                        "invalid shards file, using builtin map list"
                    );
                    Self::builtin()
                }
            },
            Err(_) => Self::builtin(),
        }
    }

    pub fn maps(&self) -> Vec<u32> {
        self.shards
            .iter()
            .flat_map(|shard| shard.maps.iter().copied())
            .collect()
    }

    pub fn shard_for_map(&self, map_id: u32) -> Option<&ShardSpec> {
        self.shards
            .iter()
            .find(|shard| shard.maps.contains(&map_id))
    }
}

#[derive(Debug, Deserialize)]
struct RawFile {
    shards: Vec<RawShard>,
}

#[derive(Debug, Deserialize)]
struct RawShard {
    name: String,
    maps: Vec<u32>,
    #[serde(default)]
    dedicated: bool,
    #[serde(default)]
    kind: Option<String>,
}

#[cfg(test)]
#[path = "../test/shards.rs"]
mod tests;
