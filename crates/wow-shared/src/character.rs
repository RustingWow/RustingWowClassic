use serde::{Deserialize, Serialize};

use crate::credentials::Account;
use crate::map::MAP_EASTERN_KINGDOMS;

pub const NORTHSHIRE_X: f32 = -8949.95;
pub const NORTHSHIRE_Y: f32 = -132.493;
pub const NORTHSHIRE_Z: f32 = 83.5312;
pub const NORTHSHIRE_ORIENTATION: f32 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

impl Position {
    pub const NORTHSHIRE: Self = Self {
        x: NORTHSHIRE_X,
        y: NORTHSHIRE_Y,
        z: NORTHSHIRE_Z,
        orientation: NORTHSHIRE_ORIENTATION,
    };

    pub fn northshire_for_account(account_id: i64) -> Self {
        let offset = u32::try_from(account_id.max(1)).unwrap_or(u32::MAX);
        Self {
            y: NORTHSHIRE_Y + offset.saturating_sub(1) as f32 * 3.0,
            ..Self::NORTHSHIRE
        }
    }

    pub fn distance_squared(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

/// Hardcoded Human Warrior spawned at Northshire Abbey.
#[derive(Debug, Clone)]
pub struct CharacterTemplate {
    pub guid: u64,
    pub name: String,
    pub map_id: u32,
    pub position: Position,
}

impl CharacterTemplate {
    pub fn for_account(account: &Account) -> Self {
        Self {
            guid: account.id as u64,
            name: format!("User{}", account.id),
            map_id: MAP_EASTERN_KINGDOMS,
            position: Position::northshire_for_account(account.id),
        }
    }
}
