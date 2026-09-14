use std::sync::Arc;

use crate::accounts::{AccountStore, CreateAccountError, VerifyAccountError};
use crate::store::SessionStore;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use tokio::net::TcpListener;

const INTERNAL_TOKEN_HEADER: &str = "x-auth-internal-token";

#[derive(Clone)]
struct HttpState {
    sessions: SessionStore,
    accounts: AccountStore,
    internal_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateAccountBody {
    username: String,
    password: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyAccountBody {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct SetGmLevelBody {
    username: String,
    gmlevel: u8,
}

#[derive(serde::Serialize)]
struct ErrorBody {
    error: &'static str,
}

pub async fn serve(
    listener: TcpListener,
    sessions: SessionStore,
    accounts: AccountStore,
    internal_token: Option<String>,
) -> anyhow::Result<()> {
    let app = router(HttpState {
        sessions,
        accounts,
        internal_token,
    });

    axum::serve(listener, app).await?;
    Ok(())
}

fn router(state: HttpState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/internal/sessions/{account}", get(get_session))
        .route("/internal/accounts", post(create_account))
        .route("/internal/accounts/verify", post(verify_account))
        .route("/internal/accounts/gm", get(list_gms))
        .route("/internal/accounts/gmlevel", put(set_gmlevel))
        .route("/internal/accounts/{id}", get(get_account))
        .with_state(Arc::new(state))
}

async fn health() -> &'static str {
    "ok"
}

async fn get_session(
    Path(account): Path<String>,
    State(state): State<Arc<HttpState>>,
) -> impl IntoResponse {
    match state.sessions.get(&account) {
        Some(info) => Json(info).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn create_account(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Json(body): Json<CreateAccountBody>,
) -> impl IntoResponse {
    if let Err(response) = require_internal_token(&headers, &state.internal_token) {
        return response;
    }

    match state
        .accounts
        .create(&body.username, &body.password, body.email.as_deref())
        .await
    {
        Ok(profile) => (StatusCode::CREATED, Json(profile)).into_response(),
        Err(CreateAccountError::InvalidUsername) => error_response(
            StatusCode::BAD_REQUEST,
            "Username must be 2–16 letters or digits.",
        ),
        Err(CreateAccountError::InvalidPassword) => {
            error_response(StatusCode::BAD_REQUEST, "Password must be 4–16 characters.")
        }
        Err(CreateAccountError::InvalidEmail) => {
            error_response(StatusCode::BAD_REQUEST, "Email is not valid.")
        }
        Err(CreateAccountError::DuplicateUsername) => {
            error_response(StatusCode::CONFLICT, "That username is already taken.")
        }
        Err(CreateAccountError::Store(error)) => {
            tracing::error!(?error, "failed to create account");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create account.",
            )
        }
    }
}

async fn verify_account(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Json(body): Json<VerifyAccountBody>,
) -> impl IntoResponse {
    if let Err(response) = require_internal_token(&headers, &state.internal_token) {
        return response;
    }

    match state.accounts.verify(&body.username, &body.password).await {
        Ok(profile) => Json(profile).into_response(),
        Err(VerifyAccountError::InvalidCredentials) => {
            error_response(StatusCode::UNAUTHORIZED, "Invalid username or password.")
        }
        Err(VerifyAccountError::Locked) => {
            error_response(StatusCode::FORBIDDEN, "This account is locked.")
        }
        Err(VerifyAccountError::Store(error)) => {
            tracing::error!(?error, "failed to verify account");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify account.",
            )
        }
    }
}

async fn get_account(
    Path(id): Path<i64>,
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(response) = require_internal_token(&headers, &state.internal_token) {
        return response;
    }

    match state.accounts.get_by_id(id).await {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(?error, "failed to load account");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load account.")
        }
    }
}

async fn set_gmlevel(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Json(body): Json<SetGmLevelBody>,
) -> impl IntoResponse {
    if let Err(response) = require_internal_token(&headers, &state.internal_token) {
        return response;
    }
    if body.gmlevel > 3 {
        return error_response(StatusCode::BAD_REQUEST, "gmlevel must be 0–3.");
    }
    match state
        .accounts
        .set_gmlevel(&body.username, body.gmlevel)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(?error, "failed to set gmlevel");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to set gmlevel.",
            )
        }
    }
}

async fn list_gms(State(state): State<Arc<HttpState>>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(response) = require_internal_token(&headers, &state.internal_token) {
        return response;
    }
    match state.accounts.list_gms().await {
        Ok(gms) => Json(
            gms.into_iter()
                .map(|(username, gmlevel)| {
                    serde_json::json!({ "username": username, "gmlevel": gmlevel })
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => {
            tracing::error!(?error, "failed to list gms");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list GM accounts.",
            )
        }
    }
}

fn require_internal_token(
    headers: &HeaderMap,
    expected: &Option<String>,
) -> Result<(), axum::response::Response> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let provided = headers
        .get(INTERNAL_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok());
    if provided == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Missing or invalid internal token.",
        ))
    }
}

fn error_response(status: StatusCode, error: &'static str) -> axum::response::Response {
    (status, Json(ErrorBody { error })).into_response()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{Value, json};
    use tokio::net::TcpListener;
    use tokio::time::sleep;

    use super::{HttpState, router};
    use crate::accounts::AccountStore;
    use crate::store::SessionStore;

    #[tokio::test]
    async fn account_api_create_verify_and_get() {
        let addr = spawn_server(Some("secret".into())).await;
        let client = reqwest::Client::new();
        let token = "secret";

        let created = client
            .post(format!("http://{addr}/internal/accounts"))
            .header("X-Auth-Internal-Token", token)
            .json(&json!({
                "username": "alice",
                "password": "secret123",
                "email": "alice@example.com"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(created.status(), 201);
        let profile: Value = created.json().await.unwrap();
        assert_eq!(profile["username"], "ALICE");
        let id = profile["id"].as_i64().unwrap();

        let duplicate = client
            .post(format!("http://{addr}/internal/accounts"))
            .header("X-Auth-Internal-Token", token)
            .json(&json!({ "username": "ALICE", "password": "secret123" }))
            .send()
            .await
            .unwrap();
        assert_eq!(duplicate.status(), 409);

        let verified = client
            .post(format!("http://{addr}/internal/accounts/verify"))
            .header("X-Auth-Internal-Token", token)
            .json(&json!({ "username": "alice", "password": "secret123" }))
            .send()
            .await
            .unwrap();
        assert_eq!(verified.status(), 200);

        let wrong = client
            .post(format!("http://{addr}/internal/accounts/verify"))
            .header("X-Auth-Internal-Token", token)
            .json(&json!({ "username": "alice", "password": "nope" }))
            .send()
            .await
            .unwrap();
        assert_eq!(wrong.status(), 401);

        let loaded = client
            .get(format!("http://{addr}/internal/accounts/{id}"))
            .header("X-Auth-Internal-Token", token)
            .send()
            .await
            .unwrap();
        assert_eq!(loaded.status(), 200);
        let body: Value = loaded.json().await.unwrap();
        assert_eq!(body["email"], "alice@example.com");
    }

    #[tokio::test]
    async fn account_api_requires_token_and_respects_lock() {
        let accounts = AccountStore::memory();
        accounts.create("bob", "secret123", None).await.unwrap();
        accounts.lock_for_test("bob");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                router(HttpState {
                    sessions: SessionStore::memory(),
                    accounts,
                    internal_token: Some("secret".into()),
                }),
            )
            .await
            .unwrap();
        });
        wait_for_health(&format!("http://{addr}")).await;

        let client = reqwest::Client::new();
        let unauthorized = client
            .post(format!("http://{addr}/internal/accounts/verify"))
            .json(&serde_json::json!({ "username": "bob", "password": "secret123" }))
            .send()
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), 401);

        let locked = client
            .post(format!("http://{addr}/internal/accounts/verify"))
            .header("X-Auth-Internal-Token", "secret")
            .json(&serde_json::json!({ "username": "bob", "password": "secret123" }))
            .send()
            .await
            .unwrap();
        assert_eq!(locked.status(), 403);

        let session = client
            .get(format!("http://{addr}/internal/sessions/missing"))
            .send()
            .await
            .unwrap();
        assert_eq!(session.status(), 404);
    }

    async fn spawn_server(token: Option<String>) -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                router(HttpState {
                    sessions: SessionStore::memory(),
                    accounts: AccountStore::memory(),
                    internal_token: token,
                }),
            )
            .await
            .unwrap();
        });
        wait_for_health(&format!("http://{addr}")).await;
        addr
    }

    async fn wait_for_health(url: &str) {
        let client = reqwest::Client::new();
        for _ in 0..50 {
            if client
                .get(format!("{url}/health"))
                .send()
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false)
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
        panic!("auth HTTP did not become ready at {url}");
    }
}
