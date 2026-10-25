use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

use crate::AppState;

/// GET /health — DB への疎通まで含めて確認する。
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok", "db": "ok" }))),
        Err(err) => {
            tracing::error!(error = %err, "health check: database unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "error", "db": "unavailable" })),
            )
        }
    }
}
