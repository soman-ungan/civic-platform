mod health;

use axum::{Router, routing::get};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health::health))
}
