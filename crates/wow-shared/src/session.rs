use serde::{Deserialize, Serialize};
use wow_srp::SESSION_KEY_LENGTH;

pub const SESSION_KEY_LEN: usize = SESSION_KEY_LENGTH as usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub account: String,
    #[serde(with = "session_key_serde")]
    pub session_key: [u8; SESSION_KEY_LEN],
}

mod session_key_serde {
    use super::SESSION_KEY_LEN;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        key: &[u8; SESSION_KEY_LEN],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        key.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; SESSION_KEY_LEN], D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes.try_into().map_err(|bytes: Vec<u8>| {
            serde::de::Error::custom(format!(
                "expected {SESSION_KEY_LEN} session-key bytes, got {}",
                bytes.len()
            ))
        })
    }
}
