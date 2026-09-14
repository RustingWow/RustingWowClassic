use wow_shared::Position;

const GAMEOBJECT_HIGH: u64 = 0xF110;

pub const GO_TYPE_TRANSPORT: i32 = 11;
pub const GO_TYPE_MO_TRANSPORT: i32 = 15;

#[cfg(test)]
pub const ENTRY_NORTHSHIRE_CHEST: u32 = 32;
#[cfg(test)]
pub const DISPLAY_NORTHSHIRE_CHEST: i32 = 336;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameObject {
    pub guid: u64,
    pub entry: u32,
    pub name: String,
    pub display_id: i32,
    pub object_type: i32,
    pub flags: i32,
    pub faction: i32,
    pub state: i32,
    pub anim_progress: i32,
    pub scale: f32,
    pub position: Position,
    #[allow(dead_code)]
    pub rotation: [f32; 4],
    pub data: [i32; 24],
}

impl GameObject {
    pub fn is_static_spawn(object_type: i32) -> bool {
        object_type != GO_TYPE_TRANSPORT && object_type != GO_TYPE_MO_TRANSPORT
    }

    pub fn query_data(&self) -> [u32; 6] {
        std::array::from_fn(|i| self.data[i] as u32)
    }
}

pub fn gameobject_guid(counter: u32, entry: u32) -> u64 {
    u64::from(counter) | (u64::from(entry) << 24) | (GAMEOBJECT_HIGH << 48)
}

pub fn is_gameobject_guid(guid: u64) -> bool {
    guid >> 48 == GAMEOBJECT_HIGH
}

#[cfg(test)]
pub fn northshire_chest_guid() -> u64 {
    gameobject_guid(1, ENTRY_NORTHSHIRE_CHEST)
}

#[cfg(test)]
pub fn northshire_chest() -> GameObject {
    GameObject {
        guid: northshire_chest_guid(),
        entry: ENTRY_NORTHSHIRE_CHEST,
        name: "Old Chest".to_string(),
        display_id: DISPLAY_NORTHSHIRE_CHEST,
        object_type: 3,
        flags: 0,
        faction: 0,
        state: 1,
        anim_progress: 100,
        scale: 1.0,
        position: Position {
            x: Position::NORTHSHIRE.x + 4.0,
            y: Position::NORTHSHIRE.y + 4.0,
            z: Position::NORTHSHIRE.z,
            orientation: 0.0,
        },
        rotation: [0.0, 0.0, 0.0, 1.0],
        data: [0; 24],
    }
}
