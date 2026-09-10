use serde::{Deserialize, Serialize};

use crate::enums::{
    CharacterArea, CharacterClass, CharacterGender, CharacterMap, CharacterRace,
};

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

    pub fn distance_squared(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn cell(self, size: f32) -> (i32, i32) {
        ((self.x / size).floor() as i32, (self.y / size).floor() as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Appearance {
    pub skin: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_hair: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterTemplate {
    pub guid: u64,
    pub name: String,
    pub race: CharacterRace,
    pub class: CharacterClass,
    pub gender: CharacterGender,
    pub appearance: Appearance,
    pub map_id: CharacterMap,
    pub position: Position,
    pub area: CharacterArea,
    pub first_login: bool,
}
