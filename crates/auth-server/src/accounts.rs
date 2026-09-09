use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Row};
use wow_shared::normalize_username;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::SrpVerifier;
use wow_srp::{PASSWORD_VERIFIER_LENGTH, SALT_LENGTH};

const SALT_LEN: usize = SALT_LENGTH as usize;
const VERIFIER_LEN: usize = PASSWORD_VERIFIER_LENGTH as usize;
const MIN_PASSWORD_LEN: usize = 4;
const MAX_PASSWORD_LEN: usize = 16;

#[derive(Debug, Clone)]
pub struct AccountRecord {
    pub id: i64,
    pub username: String,
    pub salt: [u8; SALT_LEN],
    pub verifier: [u8; VERIFIER_LEN],
    pub locked: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountProfile {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug)]
pub enum CreateAccountError {
    InvalidUsername,
    InvalidPassword,
    InvalidEmail,
    DuplicateUsername,
    Store(anyhow::Error),
}

impl std::fmt::Display for CreateAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUsername => write!(f, "invalid username"),
            Self::InvalidPassword => write!(f, "invalid password"),
            Self::InvalidEmail => write!(f, "invalid email"),
            Self::DuplicateUsername => write!(f, "username already taken"),
            Self::Store(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CreateAccountError {}

#[derive(Debug)]
pub enum VerifyAccountError {
    InvalidCredentials,
    Locked,
    Store(anyhow::Error),
}

impl std::fmt::Display for VerifyAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "invalid username or password"),
            Self::Locked => write!(f, "account is locked"),
            Self::Store(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for VerifyAccountError {}

#[derive(Clone)]
pub struct AccountStore {
    inner: AccountStoreInner,
}

#[derive(Clone)]
enum AccountStoreInner {
    Memory(MemoryAccounts),
    Postgres(PgPool),
}

#[derive(Clone)]
struct StoredAccount {
    id: i64,
    username: String,
    email: Option<String>,
    password_hash: String,
    salt: [u8; SALT_LEN],
    verifier: [u8; VERIFIER_LEN],
    locked: bool,
}

impl StoredAccount {
    fn record(&self) -> AccountRecord {
        AccountRecord {
            id: self.id,
            username: self.username.clone(),
            salt: self.salt,
            verifier: self.verifier,
            locked: self.locked,
        }
    }

    fn profile(&self) -> AccountProfile {
        AccountProfile {
            id: self.id,
            username: self.username.clone(),
            email: self.email.clone(),
        }
    }
}

#[derive(Clone)]
struct MemoryAccounts {
    by_name: Arc<DashMap<String, StoredAccount>>,
    by_id: Arc<DashMap<i64, String>>,
    next_id: Arc<AtomicI64>,
}

impl AccountStore {
    pub fn memory() -> Self {
        Self {
            inner: AccountStoreInner::Memory(MemoryAccounts {
                by_name: Arc::new(DashMap::new()),
                by_id: Arc::new(DashMap::new()),
                next_id: Arc::new(AtomicI64::new(1)),
            }),
        }
    }

    pub fn memory_with_user1() -> anyhow::Result<Self> {
        let store = Self::memory();
        match &store.inner {
            AccountStoreInner::Memory(memory) => {
                memory.insert("USER1", "PASS1", None)?;
            }
            AccountStoreInner::Postgres(_) => unreachable!("fresh memory store"),
        }
        Ok(store)
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self {
            inner: AccountStoreInner::Postgres(pool),
        }
    }

    #[cfg(test)]
    fn memory_inner(&self) -> Option<&MemoryAccounts> {
        match &self.inner {
            AccountStoreInner::Memory(memory) => Some(memory),
            AccountStoreInner::Postgres(_) => None,
        }
    }

    pub async fn get_by_username(&self, username: &str) -> anyhow::Result<Option<AccountRecord>> {
        let Ok(username) = normalize_username(username) else {
            return Ok(None);
        };
        match &self.inner {
            AccountStoreInner::Memory(memory) => {
                Ok(memory.by_name.get(&username).map(|entry| entry.record()))
            }
            AccountStoreInner::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, username, srp_salt, srp_verifier, locked
                     FROM accounts
                     WHERE username = $1",
                )
                .bind(&username)
                .fetch_optional(pool)
                .await?;
                let Some(row) = row else {
                    return Ok(None);
                };
                Ok(Some(AccountRecord {
                    id: row.try_get("id")?,
                    username: row.try_get("username")?,
                    salt: bytes_array(row.try_get::<Vec<u8>, _>("srp_salt")?)?,
                    verifier: bytes_array(row.try_get::<Vec<u8>, _>("srp_verifier")?)?,
                    locked: row.try_get("locked")?,
                }))
            }
        }
    }

    pub async fn get_by_id(&self, id: i64) -> anyhow::Result<Option<AccountProfile>> {
        match &self.inner {
            AccountStoreInner::Memory(memory) => {
                let Some(username) = memory.by_id.get(&id).map(|entry| entry.clone()) else {
                    return Ok(None);
                };
                Ok(memory.by_name.get(&username).map(|entry| entry.profile()))
            }
            AccountStoreInner::Postgres(pool) => {
                let row = sqlx::query("SELECT id, username, email FROM accounts WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;
                let Some(row) = row else {
                    return Ok(None);
                };
                Ok(Some(AccountProfile {
                    id: row.try_get("id")?,
                    username: row.try_get("username")?,
                    email: row.try_get("email")?,
                }))
            }
        }
    }

    pub async fn create(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<AccountProfile, CreateAccountError> {
        let username =
            normalize_username(username).map_err(|_| CreateAccountError::InvalidUsername)?;
        let email = normalize_email(email)?;
        validate_password(password)?;

        match &self.inner {
            AccountStoreInner::Memory(memory) => {
                memory.insert(&username, password, email.as_deref())
            }
            AccountStoreInner::Postgres(pool) => {
                let (salt, verifier) = srp_material(&username, password)?;
                let password_hash = hash_password(password)?;
                let result = sqlx::query(
                    "INSERT INTO accounts
                        (username, email, password_hash, srp_salt, srp_verifier)
                     VALUES ($1, $2, $3, $4, $5)
                     RETURNING id, username, email",
                )
                .bind(&username)
                .bind(&email)
                .bind(&password_hash)
                .bind(&salt[..])
                .bind(&verifier[..])
                .fetch_one(pool)
                .await;

                match result {
                    Ok(row) => Ok(AccountProfile {
                        id: row.try_get("id").map_err(CreateAccountError::store)?,
                        username: row.try_get("username").map_err(CreateAccountError::store)?,
                        email: row.try_get("email").map_err(CreateAccountError::store)?,
                    }),
                    Err(error) if is_unique_violation(&error) => {
                        Err(CreateAccountError::DuplicateUsername)
                    }
                    Err(error) => Err(CreateAccountError::Store(error.into())),
                }
            }
        }
    }

    pub async fn verify(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AccountProfile, VerifyAccountError> {
        let Ok(username) = normalize_username(username) else {
            return Err(VerifyAccountError::InvalidCredentials);
        };
        if password.is_empty() {
            return Err(VerifyAccountError::InvalidCredentials);
        }

        match &self.inner {
            AccountStoreInner::Memory(memory) => {
                let Some(account) = memory.by_name.get(&username).map(|entry| entry.clone()) else {
                    return Err(VerifyAccountError::InvalidCredentials);
                };
                finish_verify(&account, password)
            }
            AccountStoreInner::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, username, email, password_hash, locked
                     FROM accounts
                     WHERE username = $1",
                )
                .bind(&username)
                .fetch_optional(pool)
                .await
                .map_err(VerifyAccountError::store)?;
                let Some(row) = row else {
                    return Err(VerifyAccountError::InvalidCredentials);
                };
                let account = StoredAccount {
                    id: row.try_get("id").map_err(VerifyAccountError::store)?,
                    username: row.try_get("username").map_err(VerifyAccountError::store)?,
                    email: row.try_get("email").map_err(VerifyAccountError::store)?,
                    password_hash: row
                        .try_get("password_hash")
                        .map_err(VerifyAccountError::store)?,
                    salt: [0; SALT_LEN],
                    verifier: [0; VERIFIER_LEN],
                    locked: row.try_get("locked").map_err(VerifyAccountError::store)?,
                };
                finish_verify(&account, password)
            }
        }
    }

    #[cfg(test)]
    pub fn lock_for_test(&self, username: &str) {
        let username = normalize_username(username).expect("valid username");
        let memory = self.memory_inner().expect("memory store");
        if let Some(mut account) = memory.by_name.get_mut(&username) {
            account.locked = true;
        }
    }
}

impl MemoryAccounts {
    fn insert(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<AccountProfile, CreateAccountError> {
        let username =
            normalize_username(username).map_err(|_| CreateAccountError::InvalidUsername)?;
        if self.by_name.contains_key(&username) {
            return Err(CreateAccountError::DuplicateUsername);
        }
        let email = normalize_email(email)?;
        validate_password(password)?;
        let (salt, verifier) = srp_material(&username, password)?;
        let password_hash = hash_password(password)?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let stored = StoredAccount {
            id,
            username: username.clone(),
            email: email.clone(),
            password_hash,
            salt,
            verifier,
            locked: false,
        };
        let profile = stored.profile();
        self.by_name.insert(username.clone(), stored);
        self.by_id.insert(id, username);
        Ok(profile)
    }
}

impl CreateAccountError {
    fn store(error: impl Into<anyhow::Error>) -> Self {
        Self::Store(error.into())
    }
}

impl VerifyAccountError {
    fn store(error: impl Into<anyhow::Error>) -> Self {
        Self::Store(error.into())
    }
}

fn finish_verify(
    account: &StoredAccount,
    password: &str,
) -> Result<AccountProfile, VerifyAccountError> {
    if account.locked {
        return Err(VerifyAccountError::Locked);
    }
    if !verify_password(password, &account.password_hash) {
        return Err(VerifyAccountError::InvalidCredentials);
    }
    Ok(account.profile())
}

fn validate_password(password: &str) -> Result<(), CreateAccountError> {
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&password.len()) {
        return Err(CreateAccountError::InvalidPassword);
    }
    Ok(())
}

fn normalize_email(email: Option<&str>) -> Result<Option<String>, CreateAccountError> {
    let Some(raw) = email else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !is_valid_email(trimmed) {
        return Err(CreateAccountError::InvalidEmail);
    }
    Ok(Some(trimmed.to_string()))
}

fn is_valid_email(email: &str) -> bool {
    let Some((user, domain)) = email.split_once('@') else {
        return false;
    };
    !user.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn srp_material(
    username: &str,
    password: &str,
) -> Result<([u8; SALT_LEN], [u8; VERIFIER_LEN]), CreateAccountError> {
    let user = NormalizedString::new(username).map_err(|_| CreateAccountError::InvalidUsername)?;
    let pass = NormalizedString::new(password).map_err(|_| CreateAccountError::InvalidPassword)?;
    let verifier = SrpVerifier::from_username_and_password(user, pass);
    Ok((*verifier.salt(), *verifier.password_verifier()))
}

fn hash_password(password: &str) -> Result<String, CreateAccountError> {
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| CreateAccountError::Store(anyhow::anyhow!("argon2 hash: {error}")))
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| code == "23505")
}

fn bytes_array<const N: usize>(bytes: Vec<u8>) -> anyhow::Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| anyhow::anyhow!("expected {N} bytes, got {}", bytes.len()))
}

pub async fn connect_auth(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute("CREATE SCHEMA IF NOT EXISTS auth").await?;
                conn.execute("SET search_path TO auth").await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::AccountStore;
    use wow_srp::PublicKey;
    use wow_srp::client::SrpClientChallenge;
    use wow_srp::normalized_string::NormalizedString;
    use wow_srp::server::SrpVerifier;
    use wow_srp::{GENERATOR, LARGE_SAFE_PRIME_LITTLE_ENDIAN};

    #[tokio::test]
    async fn memory_store_seeds_user1() {
        let store = AccountStore::memory_with_user1().unwrap();
        let account = store.get_by_username("user1").await.unwrap().unwrap();
        assert_eq!(account.id, 1);
        assert_eq!(account.username, "USER1");
        assert!(!account.locked);
    }

    #[tokio::test]
    async fn create_and_verify_memory_account() {
        let store = AccountStore::memory();
        let created = store
            .create("alice", "secret123", Some("alice@example.com"))
            .await
            .unwrap();
        assert_eq!(created.username, "ALICE");
        assert_eq!(created.email.as_deref(), Some("alice@example.com"));

        let verified = store.verify("alice", "secret123").await.unwrap();
        assert_eq!(verified.id, created.id);
        assert!(store.verify("alice", "wrongpass").await.is_err());
    }

    #[tokio::test]
    async fn duplicate_username_is_rejected() {
        let store = AccountStore::memory();
        store.create("bob", "secret123", None).await.unwrap();
        let err = store.create("BOB", "secret123", None).await.unwrap_err();
        assert!(matches!(err, super::CreateAccountError::DuplicateUsername));
    }

    #[tokio::test]
    async fn created_account_authenticates_with_wow_srp() {
        let store = AccountStore::memory();
        store.create("alice", "secret123", None).await.unwrap();
        let account = store.get_by_username("alice").await.unwrap().unwrap();
        let username = NormalizedString::new("ALICE").unwrap();
        let password = NormalizedString::new("secret123").unwrap();
        let proof =
            SrpVerifier::from_database_values(username.clone(), account.verifier, account.salt)
                .into_proof();
        let client = SrpClientChallenge::new(
            username,
            password,
            GENERATOR,
            LARGE_SAFE_PRIME_LITTLE_ENDIAN,
            PublicKey::from_le_bytes(*proof.server_public_key()).unwrap(),
            *proof.salt(),
        );
        proof
            .into_server(
                PublicKey::from_le_bytes(*client.client_public_key()).unwrap(),
                *client.client_proof(),
            )
            .expect("created salt/verifier must authenticate");
    }

    #[test]
    fn node_golden_verifier_works_with_wow_srp_client() {
        let salt = be_hex_to_le("AFE5D28E925DBB3DAFED5D91ACA0928940E8FBFEF2D2A3CC154ADA0FE6ABEF6F");
        let verifier =
            be_hex_to_le("21B4153B0A938D0A69D28F2690CC3F79A99A13C40CACB525B3B79D4201EB33FF");
        let username = NormalizedString::new("LF2BGFQIFQ3HZ1ZF").unwrap();
        let password = NormalizedString::new("MVRVMUJFWRA0IBVK").unwrap();
        let proof =
            SrpVerifier::from_database_values(username.clone(), verifier, salt).into_proof();
        let client = SrpClientChallenge::new(
            username,
            password,
            GENERATOR,
            LARGE_SAFE_PRIME_LITTLE_ENDIAN,
            PublicKey::from_le_bytes(*proof.server_public_key()).unwrap(),
            *proof.salt(),
        );
        proof
            .into_server(
                PublicKey::from_le_bytes(*client.client_public_key()).unwrap(),
                *client.client_proof(),
            )
            .expect("golden salt/verifier must authenticate");
    }

    fn be_hex_to_le(hex: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[31 - i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }
}
