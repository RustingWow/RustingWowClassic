use std::sync::Arc;

use crate::store::SessionStore;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tokio::net::TcpListener;

pub async fn serve(listener: TcpListener, store: SessionStore) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/internal/sessions/{account}", get(get_session))
        .with_state(Arc::new(store));

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn get_session(
    Path(account): Path<String>,
    State(store): State<Arc<SessionStore>>,
) -> impl IntoResponse {
    match store.get(&account) {
        Some(info) => Json(info).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
