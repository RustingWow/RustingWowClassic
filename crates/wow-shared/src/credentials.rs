use thiserror::Error;

/// Registered game/website account. Password material lives in Postgres, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    pub username: String,
    /// CMaNGOS security: 0 player, 1 moderator, 2 GM, 3 admin.
    pub gmlevel: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialError {
    #[error("account '{0}' is not a valid username")]
    InvalidUsername(String),
}

/// Uppercases and checks WoW username rules: 2–16 ASCII alphanumeric characters.
pub fn normalize_username(username: &str) -> Result<String, CredentialError> {
    let normalized = username.trim().to_ascii_uppercase();
    if !(2..=16).contains(&normalized.len())
        || !normalized.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(CredentialError::InvalidUsername(username.to_string()));
    }
    Ok(normalized)
}

#[cfg(test)]
#[path = "../test/credentials.rs"]
mod tests;
