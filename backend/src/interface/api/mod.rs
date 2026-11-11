// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP API（すべて `/api` の下）。アプリケーションの状態（`AppState`）とルーターを組み立てる

pub mod analysis;
pub mod auth;
pub mod digest;
pub mod error;
pub mod finances;
mod health;
pub mod municipalities;
pub mod officials;
pub mod operator;
pub mod participant;
pub mod participation;
pub mod proposals;
pub mod rate_limit;
pub mod topics;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::extract::FromRef;
use axum::routing::{get, patch, post, put};
use axum_extra::extract::cookie::Key;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::process::moderation::Moderation;
use crate::process::progress;
use error::ApiError;

/// すべての API は `/api` の下に置く（本番ではリバースプロキシが `/api` を backend に振り分ける）。
pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/health", get(health::health))
        .route("/auth/me", get(auth::me).delete(auth::delete_me))
        .route("/auth/register/start", post(auth::register_start))
        .route("/auth/register/finish", post(auth::register_finish))
        .route("/auth/login/start", post(auth::login_start))
        .route("/auth/login/finish", post(auth::login_finish))
        .route("/auth/logout", post(auth::logout))
        .route("/municipalities", get(municipalities::list))
        .route("/municipalities/{slug}", get(municipalities::detail))
        .route(
            "/municipalities/{slug}/relation",
            put(participation::put_relation),
        )
        .route("/municipalities/{slug}/finances", get(finances::finances))
        .route(
            "/municipalities/{slug}/proposals",
            get(proposals::mine).post(proposals::create),
        )
        .route("/municipalities/{slug}/topic-map", get(digest::map))
        .route("/municipalities/{slug}/digest", get(digest::digest))
        .route("/municipalities/{slug}/officials", get(officials::list))
        .route(
            "/municipalities/{slug}/officials/{official_id}",
            get(officials::detail),
        )
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
        .nest("/operator", operator_routes())
        .fallback(|| async { ApiError::NotFound });
    Router::new().nest("/api", api)
}

/// 運営者の API（`/api/operator` の下。運営者でなければすべて 404）
fn operator_routes() -> Router<AppState> {
    const M: &str = "/municipalities/{slug}";
    Router::new()
        .route("/municipalities", get(operator::municipalities))
        .route(&format!("{M}/tags"), get(operator::tag_choices))
        .route(&format!("{M}/topics"), get(operator::topics::list_topics))
        .route(
            &format!("{M}/topics/{{topic_id}}"),
            get(operator::topics::topic).patch(operator::topics::edit_topic),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/publish"),
            post(operator::topics::publish_topic),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/archive"),
            post(operator::topics::archive_topic),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/closes-at"),
            put(operator::topics::set_closes_at),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/queue"),
            post(operator::publishing::enqueue).delete(operator::publishing::dequeue),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/queue/move"),
            post(operator::publishing::move_queued),
        )
        .route(
            &format!("{M}/publishing"),
            get(operator::publishing::publishing),
        )
        .route(
            &format!("{M}/publish-slots"),
            post(operator::publishing::add_slot),
        )
        .route(
            &format!("{M}/publish-slots/{{slot_id}}"),
            patch(operator::publishing::update_slot).delete(operator::publishing::delete_slot),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/statements"),
            post(operator::topics::add_statement),
        )
        .route(
            &format!("{M}/topics/{{topic_id}}/tags/{{tag_id}}"),
            put(operator::topics::put_tag).delete(operator::topics::delete_tag),
        )
        .route(&format!("{M}/statements"), get(operator::statements::queue))
        .route(
            &format!("{M}/statements/{{statement_id}}"),
            patch(operator::statements::edit_statement),
        )
        .route(
            &format!("{M}/statements/{{statement_id}}/approve"),
            post(operator::statements::approve),
        )
        .route(
            &format!("{M}/statements/{{statement_id}}/reject"),
            post(operator::statements::reject),
        )
        .route(
            &format!("{M}/statements/{{statement_id}}/retry"),
            post(operator::statements::retry),
        )
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
    /// パスキーの登録・ログイン（ADR 0006）
    pub webauthn: std::sync::Arc<webauthn_rs::Webauthn>,
    /// 書き込みの回数の制限（本番では main が有効にする。テストでは数えない）
    pub rate_limiter: std::sync::Arc<rate_limit::RateLimiter>,
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
            // 開発用の既定（localhost）。main では設定（WEBAUTHN_RP_ID・WEBAUTHN_ORIGIN）で差し替える
            webauthn: auth::webauthn("localhost", "http://localhost:3000")?,
            rate_limiter: std::sync::Arc::new(rate_limit::RateLimiter::disabled()),
        })
    }
}

impl AppState {
    /// 投稿の判定方式を差し替える（API キーがあるときの LLM、テストの偽の LLM）
    pub fn with_moderation(mut self, moderation: Moderation) -> Self {
        self.moderation = moderation;
        self
    }

    /// 書き込みの回数の制限を差し替える
    pub fn with_rate_limiter(mut self, limiter: rate_limit::RateLimiter) -> Self {
        self.rate_limiter = std::sync::Arc::new(limiter);
        self
    }

    /// パスキーを結び付けるドメインと画面のオリジンを差し替える
    pub fn with_webauthn(mut self, rp_id: &str, origin: &str) -> anyhow::Result<Self> {
        self.webauthn = auth::webauthn(rp_id, origin)?;
        Ok(self)
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

/// トピックに参加できなければ 404（ルールは `progress::topic_open`）
pub(crate) async fn ensure_topic(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
) -> Result<(), ApiError> {
    if progress::topic_open(pool, municipality_id, topic_id).await? {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

/// 送ってくる本文の上限。どの API の入力も小さい（意見は140字、パスキーの応答も数KB）
const BODY_LIMIT_BYTES: usize = 64 * 1024;

/// ルーターを組み立てる。main とテストの両方から使う。
pub fn app(state: AppState) -> Router {
    router()
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit::limit,
        ))
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
