use super::DbEnum;

/// Vanilla 1.12 area (zone/subzone). Same type the client uses on the wire.
pub type CharacterArea = wow_world_base::vanilla::Area;

impl DbEnum for CharacterArea {
    fn as_protocol(self) -> u32 {
        self.as_int()
    }

    fn as_str(self) -> &'static str {
        self.as_test_case_value()
    }

    fn from_protocol(value: u32) -> Option<Self> {
        Self::from_int(value).ok()
    }

    fn from_str(value: &str) -> Option<Self> {
        area_by_db().get(value).copied()
    }
}

fn area_by_db() -> &'static std::collections::HashMap<&'static str, CharacterArea> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, CharacterArea>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        CharacterArea::variants()
            .into_iter()
            .map(|area| (area.as_test_case_value(), area))
            .collect()
    })
}
