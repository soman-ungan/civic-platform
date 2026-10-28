// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP API（すべて `/api` の下）。アプリケーションの状態（`AppState`）とルーターを組み立てる

pub mod analysis;
pub mod error;
pub mod finances;
mod health;
pub mod municipalities;
pub mod participant;
pub mod participation;
pub mod topics;

use axum::Router;
use axum::extract::FromRef;
use axum::routing::{get, post, put};
use axum_extra::extract::cookie::Key;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::moderation::Moderation;
use error::ApiError;

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
        .route("/municipalities/{slug}/finances", get(finances::finances))
        .route("/municipalities/{slug}/topics", get(topics::list))
        .route(
            "/municipalities/{slug}/topics/{topic_id}",
            get(topics::detail),
        )
        .route(
            "/municipalities/{slug}/consensus",
            get(analysis::municipality_consensus),
        )
        .route(
            "/municipalities/{slug}/topics/{topic_id}/analysis",
            get(analysis::topic_analysis),
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

/// `SESSION_SECRET` に必要な最小の長さ（バイト）
pub const MIN_SESSION_SECRET_LEN: usize = 32;

/// ハンドラ間で共有するアプリケーション状態。
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    /// 参加者 Cookie の署名鍵
    pub cookie_key: Key,
    /// Cookie に Secure 属性を付けるか（本番の HTTPS 環境で true）
    pub cookie_secure: bool,
    /// 投稿された意見の判定方式
    pub moderation: Moderation,
}

impl AppState {
    pub fn new(
        pool: SqlitePool,
        session_secret: &str,
        cookie_secure: bool,
    ) -> anyhow::Result<Self> {
        if session_secret.len() < MIN_SESSION_SECRET_LEN {
            anyhow::bail!(
                "SESSION_SECRET は {MIN_SESSION_SECRET_LEN} バイト以上にしてください（例: openssl rand -hex 32）"
            );
        }
        Ok(Self {
            pool,
            cookie_key: Key::derive_from(session_secret.as_bytes()),
            cookie_secure,
            moderation: Moderation::default(),
        })
    }
}

impl AppState {
    /// 投稿の判定方式を差し替える（API キーがあるときの LLM、テストの偽の LLM）
    pub fn with_moderation(mut self, moderation: Moderation) -> Self {
        self.moderation = moderation;
        self
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

/// ルーターを組み立てる。main とテストの両方から使う。
pub fn app(state: AppState) -> Router {
    router().layer(TraceLayer::new_for_http()).with_state(state)
}
