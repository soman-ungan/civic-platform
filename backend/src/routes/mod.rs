// SPDX-License-Identifier: AGPL-3.0-only

mod health;
pub mod municipalities;
pub mod participation;
pub mod topics;

use axum::Router;
use axum::routing::{get, post, put};

use crate::AppState;
use crate::error::ApiError;

/// すべての API は `/api` の下に置く（本番ではリバースプロキシが `/api` を backend に振り分ける）。
pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/health", get(health::health))
        .route("/municipalities", get(municipalities::list))
        .route("/municipalities/{slug}", get(municipalities::detail))
        .route(
            "/municipalities/{slug}/relation",
            put(participation::put_relation),
        )
        .route("/municipalities/{slug}/topics", get(topics::list))
        .route(
            "/municipalities/{slug}/topics/{topic_id}",
            get(topics::detail),
        )
        .route(
            "/municipalities/{slug}/topics/{topic_id}/next-statement",
            get(participation::next_statement),
        )
        .route(
            "/municipalities/{slug}/topics/{topic_id}/statements",
            post(participation::post_statement),
        )
        .route(
            "/municipalities/{slug}/topics/{topic_id}/statements/{statement_id}/votes",
            post(participation::vote),
        )
        .fallback(|| async { ApiError::NotFound });
    Router::new().nest("/api", api)
}
