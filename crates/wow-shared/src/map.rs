/// Vanilla continent and instance `map_id` values (loading-screen boundaries).
pub const MAP_EASTERN_KINGDOMS: u32 = 0;
pub const MAP_KALIMDOR: u32 = 1;
pub const MAP_DEEPRUN_TRAM: u32 = 369;

/// Placeholder spawn used when a character transfers onto Kalimdor (no zone content yet).
pub const KALIMDOR_STUB: crate::character::Position = crate::character::Position {
    x: -2917.58,
    y: -257.98,
    z: 52.996,
    orientation: 0.0,
};
