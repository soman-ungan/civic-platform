// SPDX-License-Identifier: AGPL-3.0-or-later

mod health;

use axum::{Router, routing::get};

use crate::AppState;
use crate::error::ApiError;

/// すべての API は `/api` の下に置く（本番ではリバースプロキシが `/api` を backend に振り分ける）。
pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/health", get(health::health))
        .fallback(|| async { ApiError::NotFound });
    Router::new().nest("/api", api)
}
