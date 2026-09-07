use thiserror::Error;

/// Account accepted by the in-memory credential scheme `userN` / `passN`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub number: u32,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialError {
    #[error("account '{0}' is not a valid userN name")]
    InvalidUsername(String),
    #[error("password does not match passN for this account")]
    InvalidPassword,
}

/// Parses `userN` / `USERN` with `N >= 1`.
pub fn parse_account(username: &str) -> Result<Account, CredentialError> {
    let normalized = username.to_ascii_uppercase();
    let Some(rest) = normalized.strip_prefix("USER") else {
        return Err(CredentialError::InvalidUsername(username.to_string()));
    };
    let Ok(number) = rest.parse::<u32>() else {
        return Err(CredentialError::InvalidUsername(username.to_string()));
    };
    if number == 0 || rest.starts_with('0') && rest.len() > 1 || rest.is_empty() {
        return Err(CredentialError::InvalidUsername(username.to_string()));
    }
    Ok(Account {
        number,
        username: format!("USER{number}"),
        password: expected_password(number),
    })
}

pub fn expected_password(account_number: u32) -> String {
    format!("PASS{account_number}")
}

impl Account {
    pub fn verify_password(&self, password: &str) -> Result<(), CredentialError> {
        if password.eq_ignore_ascii_case(&self.password) {
            Ok(())
        } else {
            Err(CredentialError::InvalidPassword)
        }
    }
}

#[cfg(test)]
#[path = "../test/credentials.rs"]
mod tests;
