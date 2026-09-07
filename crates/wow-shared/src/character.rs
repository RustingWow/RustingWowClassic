use crate::credentials::Account;

pub const NORTHSHIRE_X: f32 = -8949.95;
pub const NORTHSHIRE_Y: f32 = -132.493;
pub const NORTHSHIRE_Z: f32 = 83.5312;
pub const NORTHSHIRE_ORIENTATION: f32 = 0.0;

/// Hardcoded Human Warrior spawned at Northshire Abbey.
#[derive(Debug, Clone)]
pub struct CharacterTemplate {
    pub guid: u64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

impl CharacterTemplate {
    pub fn for_account(account: &Account) -> Self {
        Self {
            guid: u64::from(account.number),
            name: format!("User{}", account.number),
            x: NORTHSHIRE_X,
            y: NORTHSHIRE_Y,
            z: NORTHSHIRE_Z,
            orientation: NORTHSHIRE_ORIENTATION,
        }
    }
}
