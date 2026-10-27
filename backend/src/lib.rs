// SPDX-License-Identifier: AGPL-3.0-only

pub mod analysis;
pub mod collect;
pub mod config;
pub mod db;
pub mod drafts;
pub mod error;
pub mod eval;
pub mod llm;
pub mod minutes;
pub mod moderation;
pub mod participant;
pub mod routes;
pub mod seed;
pub mod tagging;
pub mod util;

use axum::Router;
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use moderation::Moderation;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

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
    routes::router()
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
