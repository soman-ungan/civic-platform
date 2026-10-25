pub mod config;
pub mod db;
pub mod routes;
pub mod seed;

use axum::Router;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

/// ハンドラ間で共有するアプリケーション状態。
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

/// ルーターを組み立てる。main とテストの両方から使う。
pub fn app(state: AppState) -> Router {
    routes::router()
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
