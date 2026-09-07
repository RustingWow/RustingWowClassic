use wow_shared::Position;
use wow_world_messages::vanilla::Vector3d;

pub fn vector3d(position: Position) -> Vector3d {
    Vector3d {
        x: position.x,
        y: position.y,
        z: position.z,
    }
}

pub fn position_from_vector(position: Vector3d, orientation: f32) -> Position {
    Position {
        x: position.x,
        y: position.y,
        z: position.z,
        orientation,
    }
}
