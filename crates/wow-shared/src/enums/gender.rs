use serde::{Deserialize, Serialize};

use super::db_enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterGender {
    Male,
    Female,
}

db_enum!(
    CharacterGender,
    u8,
    Male => 0, "MALE",
    Female => 1, "FEMALE",
);
