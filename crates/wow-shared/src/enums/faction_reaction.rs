//! Vanilla `FactionTemplate.dbc` hostility masks for client sword cursor / attackability.

use std::sync::OnceLock;

use client_data::FactionTemplates;

fn builtin() -> &'static FactionTemplates {
    static BUILTIN: OnceLock<FactionTemplates> = OnceLock::new();
    BUILTIN.get_or_init(FactionTemplates::builtin)
}

/// True when the 1.12 client treats this FactionTemplate as hostile to players
/// (`ourMask` includes Monster, or `hostileMask` includes Player).
pub fn hostile_to_players(template: i32) -> bool {
    builtin().hostile_to_players(template)
}

/// If `template` is already player-hostile, return it; wolf 32 becomes 38.
/// City factions stay unchanged; other neutrals become Monster 14.
pub fn ensure_hostile_to_players(template: i32) -> i32 {
    builtin().ensure_hostile_to_players(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wolf_32_is_not_client_hostile() {
        assert!(!hostile_to_players(32));
        assert_eq!(ensure_hostile_to_players(32), 38);
    }

    #[test]
    fn monster_and_wolf_38_are_client_hostile() {
        assert!(hostile_to_players(14));
        assert!(hostile_to_players(38));
        assert_eq!(ensure_hostile_to_players(14), 14);
        assert_eq!(ensure_hostile_to_players(38), 38);
    }

    #[test]
    fn stormwind_is_not_monster_hostile() {
        assert!(!hostile_to_players(11));
        assert_eq!(ensure_hostile_to_players(11), 11);
        assert_eq!(ensure_hostile_to_players(7), 14);
        assert!(!hostile_to_players(9999));
        assert_eq!(ensure_hostile_to_players(9999), 14);
    }
}
