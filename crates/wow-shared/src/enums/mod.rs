//! Protocol and database enums, split by context.

mod area;
mod class;
mod faction;
mod gender;
mod gossip;
mod map;
mod race;

pub use area::CharacterArea;
pub use class::CharacterClass;
pub use faction::CreatureFaction;
pub use gender::CharacterGender;
pub use gossip::GossipOptionKind;
pub use map::CharacterMap;
pub use race::CharacterRace;

macro_rules! db_enum {
    ($name:ident, $proto:ty, $($variant:ident => $protocol:expr, $db:expr),+ $(,)?) => {
        impl $name {
            pub const fn as_protocol(self) -> $proto {
                match self {
                    $(Self::$variant => $protocol,)+
                }
            }

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $db,)+
                }
            }

            pub fn from_protocol(value: $proto) -> Option<Self> {
                match value {
                    $($protocol => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub fn from_str(value: &str) -> Option<Self> {
                match value {
                    $($db => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

pub(crate) use db_enum;

/// Protocol id + uppercase DB label for Vanilla enums stored in Postgres.
pub trait DbEnum: Copy + Sized + 'static {
    fn as_protocol(self) -> u32;
    fn as_str(self) -> &'static str;
    fn from_protocol(value: u32) -> Option<Self>;
    fn from_str(value: &str) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_elf_hunter_matches_protocol_and_db() {
        assert_eq!(
            CharacterRace::from_protocol(4),
            Some(CharacterRace::NightElf)
        );
        assert_eq!(
            CharacterClass::from_protocol(3),
            Some(CharacterClass::Hunter)
        );
        assert_eq!(
            CharacterGender::from_protocol(0),
            Some(CharacterGender::Male)
        );
        assert_eq!(
            CharacterArea::from_protocol(188),
            Some(CharacterArea::Shadowglen)
        );
        assert_eq!(CharacterRace::NightElf.as_str(), "NIGHT_ELF");
        assert_eq!(CharacterClass::Hunter.as_str(), "HUNTER");
        assert_eq!(CharacterGender::Male.as_str(), "MALE");
        assert_eq!(CharacterArea::Shadowglen.as_str(), "SHADOWGLEN");
        assert_eq!(CharacterMap::from_protocol(1), Some(CharacterMap::Kalimdor));
        assert_eq!(CharacterMap::Kalimdor.as_str(), "KALIMDOR");
        assert_eq!(
            CharacterMap::from_protocol(0),
            Some(CharacterMap::EasternKingdoms)
        );
        assert_eq!(
            CharacterMap::from_protocol(389),
            Some(CharacterMap::RagefireChasm)
        );
        assert_eq!(CharacterMap::RagefireChasm.as_str(), "RAGEFIRE_CHASM");
        assert_eq!(
            CharacterArea::from_protocol(12),
            Some(CharacterArea::ElwynnForest)
        );
        assert_eq!(CharacterArea::ElwynnForest.as_str(), "ELWYNN_FOREST");
        assert_eq!(
            CharacterArea::from_str("UNUSED_THE_DEADMINES_002"),
            Some(CharacterArea::UnusedTheDeadmines002)
        );
        assert_eq!(
            CreatureFaction::from_protocol(11),
            Some(CreatureFaction::Stormwind)
        );
        assert_eq!(CreatureFaction::Stormwind.as_str(), "STORMWIND");
        assert_eq!(
            CreatureFaction::from_protocol(115),
            Some(CreatureFaction::PlayerGnome)
        );
        assert_eq!(CreatureFaction::PlayerGnome.as_str(), "PLAYER_GNOME");
        assert_eq!(
            CreatureFaction::from_str("MONSTER"),
            Some(CreatureFaction::Monster)
        );
        assert_eq!(CreatureFaction::Monster.as_protocol(), 14);
        assert_eq!(
            GossipOptionKind::from_protocol(3),
            Some(GossipOptionKind::Vendor)
        );
        assert_eq!(GossipOptionKind::Vendor.as_str(), "VENDOR");
        assert_eq!(
            GossipOptionKind::from_str("BOT"),
            Some(GossipOptionKind::Bot)
        );
        assert_eq!(GossipOptionKind::Bot.as_protocol(), 99);
    }
}
