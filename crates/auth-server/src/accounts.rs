use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Row};
use wow_shared::normalize_username;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::SrpVerifier;
use wow_srp::{PASSWORD_VERIFIER_LENGTH, SALT_LENGTH};

const SALT_LEN: usize = SALT_LENGTH as usize;
const VERIFIER_LEN: usize = PASSWORD_VERIFIER_LENGTH as usize;

#[derive(Debug, Clone)]
pub struct AccountRecord {
    pub id: i64,
    pub username: String,
    pub salt: [u8; SALT_LEN],
    pub verifier: [u8; VERIFIER_LEN],
    pub locked: bool,
}

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
struct MemoryAccounts {
    by_name: Arc<DashMap<String, AccountRecord>>,
    next_id: Arc<AtomicI64>,
}

impl AccountStore {
    pub fn memory_with_user1() -> anyhow::Result<Self> {
        let memory = MemoryAccounts {
            by_name: Arc::new(DashMap::new()),
            next_id: Arc::new(AtomicI64::new(1)),
        };
        memory.insert("USER1", "PASS1")?;
        Ok(Self {
            inner: AccountStoreInner::Memory(memory),
        })
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self {
            inner: AccountStoreInner::Postgres(pool),
        }
    }

    pub async fn get_by_username(&self, username: &str) -> anyhow::Result<Option<AccountRecord>> {
        let Ok(username) = normalize_username(username) else {
            return Ok(None);
        };
        match &self.inner {
            AccountStoreInner::Memory(memory) => {
                Ok(memory.by_name.get(&username).map(|entry| entry.clone()))
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
}

impl MemoryAccounts {
    fn insert(&self, username: &str, password: &str) -> anyhow::Result<AccountRecord> {
        let username = normalize_username(username)?;
        let verifier = SrpVerifier::from_username_and_password(
            NormalizedString::new(&username)?,
            NormalizedString::new(password)?,
        );
        let record = AccountRecord {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            username: username.clone(),
            salt: *verifier.salt(),
            verifier: *verifier.password_verifier(),
            locked: false,
        };
        self.by_name.insert(username, record.clone());
        Ok(record)
    }
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
    use wow_srp::normalized_string::NormalizedString;
    use wow_srp::server::SrpVerifier;
    use wow_srp::PublicKey;
    use wow_srp::client::SrpClientChallenge;
    use wow_srp::{GENERATOR, LARGE_SAFE_PRIME_LITTLE_ENDIAN};

    #[tokio::test]
    async fn memory_store_seeds_user1() {
        let store = AccountStore::memory_with_user1().unwrap();
        let account = store.get_by_username("user1").await.unwrap().unwrap();
        assert_eq!(account.id, 1);
        assert_eq!(account.username, "USER1");
        assert!(!account.locked);
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
