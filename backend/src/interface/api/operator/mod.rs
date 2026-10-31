// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の API（`/api/operator/...`。フェーズ6c。docs/plans/2026-10-29-phase6c-operator-console.md）。
//!
//! - 運営者（`users.is_operator`）だけが使える。ほかの人には、あること自体を見せないよう 404 を返す
//! - 自治体で絞り込む。トピック・意見がパスの自治体に属していなければ 404
//! - 中身は CLI と同じ `records`・`process` の関数を使う（ロジックを写さない）。判定の基準は ADR 0007
//! - 投稿者が誰かは、運営者にも返さない（市民の投稿か、議会の主張・運営者の意見かだけ）
//!
//! - `topics`: トピック（下書き・公開・期限・意見とタグ）
//! - `publishing`: 自動公開（公開待ちと公開の時刻）
//! - `statements`: 確認待ち・判定待ちの意見

pub mod publishing;
pub mod statements;
pub mod topics;

use axum::Json;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::AppState;
use super::error::ApiError;
use super::municipalities::{self, Municipality};
use super::participant::MaybeParticipant;
use crate::records::topics::{DraftDetail, EditError};
use crate::records::{accounts, statements as statement_records, tags, topics as topic_records};

/// 運営者であること。そうでなければ 404
pub struct Operator;

impl FromRequestParts<AppState> for Operator {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let MaybeParticipant(user_id) = MaybeParticipant::from_request_parts(parts, state).await?;
        let Some(user_id) = user_id else {
            return Err(ApiError::NotFound);
        };
        if accounts::status(&state.pool, &user_id).await?.operator {
            Ok(Self)
        } else {
            Err(ApiError::NotFound)
        }
    }
}

impl From<EditError> for ApiError {
    fn from(err: EditError) -> Self {
        match err {
            EditError::NotFound(_) => Self::NotFound,
            EditError::Invalid(message) => Self::validation("invalid", message),
            EditError::Database(err) => Self::Database(err),
        }
    }
}

// ===== 分けて読む =====

/// 1回に返すトピックの数（出典・意見・タグを含むので少なめ）
pub(super) const TOPICS_PER_PAGE: i64 = 10;
/// 1回に返す確認待ちの意見の数
pub(super) const QUEUE_PER_PAGE: i64 = 20;
/// `limit` の上限
const MAX_PER_PAGE: i64 = 50;

/// 一覧の続きの指定。`after` は前に返した `next`
#[derive(Debug, Default, Deserialize)]
pub struct PageQuery {
    pub after: Option<String>,
    pub limit: Option<i64>,
}

impl PageQuery {
    fn limit(&self, default: i64) -> i64 {
        self.limit.unwrap_or(default).clamp(1, MAX_PER_PAGE)
    }
}

/// 一覧の一部。`next` があれば `?after=<next>` で続きを読む
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
}

impl<T> Page<T> {
    fn new(items: Vec<T>, more: bool, id: impl Fn(&T) -> &String) -> Self {
        let next = if more {
            items.last().map(|item| id(item).clone())
        } else {
            None
        };
        Self { items, next }
    }
}

// ===== 自治体・対象の確かめ =====

/// 自治体のトピックを読む。別の自治体のもの・無いものは 404
pub(super) async fn topic_in(
    pool: &SqlitePool,
    municipality: &Municipality,
    topic_id: &str,
) -> Result<DraftDetail, ApiError> {
    topic_records::show(pool, topic_id)
        .await?
        .filter(|t| t.municipality_id == municipality.id)
        .ok_or(ApiError::NotFound)
}

// ===== 自治体・タグ =====

/// GET /api/operator/municipalities — すべての自治体（準備中のものも）
pub async fn municipalities(
    _: Operator,
    State(state): State<AppState>,
) -> Result<Json<Vec<Municipality>>, ApiError> {
    Ok(Json(municipalities::all(&state.pool).await?))
}

#[derive(Debug, Serialize)]
pub struct Tag {
    pub id: String,
    /// field / district
    pub kind: String,
    pub name: String,
}

/// GET /api/operator/municipalities/{slug}/tags — 付けられるタグ（分野と、その自治体の地区）
pub async fn tag_choices(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<Tag>>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let tags = tags::list(&state.pool, Some(&municipality.id))
        .await?
        .into_iter()
        .map(|t| Tag {
            id: t.id,
            kind: t.kind,
            name: t.name,
        })
        .collect();
    Ok(Json(tags))
}

/// 意見がこの自治体のものか確かめ、トピックの ID を返す
pub(super) async fn statement_in(
    pool: &SqlitePool,
    municipality: &Municipality,
    statement_id: &str,
) -> Result<String, ApiError> {
    match statement_records::owner(pool, statement_id).await? {
        Some((topic_id, m)) if m == municipality.id => Ok(topic_id),
        _ => Err(ApiError::NotFound),
    }
}
