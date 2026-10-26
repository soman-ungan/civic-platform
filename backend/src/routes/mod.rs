// SPDX-License-Identifier: AGPL-3.0-or-later

mod health;
pub mod municipalities;
pub mod topics;

use axum::{Router, routing::get};

use crate::AppState;
use crate::error::ApiError;

/// すべての API は `/api` の下に置く（本番ではリバースプロキシが `/api` を backend に振り分ける）。
pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/health", get(health::health))
        .route("/municipalities", get(municipalities::list))
        .route("/municipalities/{slug}", get(municipalities::detail))
        .route("/municipalities/{slug}/topics", get(topics::list))
        .route(
            "/municipalities/{slug}/topics/{topic_id}",
            get(topics::detail),
        )
        .fallback(|| async { ApiError::NotFound });
    Router::new().nest("/api", api)
}
