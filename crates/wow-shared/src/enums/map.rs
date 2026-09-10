use super::DbEnum;

/// Vanilla 1.12 continent/instance map. Same type the client uses on the wire.
pub type CharacterMap = wow_world_base::vanilla::Map;

impl DbEnum for CharacterMap {
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
        map_by_db().get(value).copied()
    }
}

fn map_by_db() -> &'static std::collections::HashMap<&'static str, CharacterMap> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, CharacterMap>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        CharacterMap::variants()
            .into_iter()
            .map(|map| (map.as_test_case_value(), map))
            .collect()
    })
}
