// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の API: 確認待ち（flagged）・判定待ち（pending）の意見（判定の基準は ADR 0007）

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Serialize;

use super::topics::StatementBody;
use super::{Operator, Page, PageQuery, QUEUE_PER_PAGE, statement_in};
use crate::interface::api::AppState;
use crate::interface::api::error::{ApiError, ApiJson};
use crate::interface::api::municipalities;
use crate::process::moderation::{self, Moderation};
use crate::records::statements;
use crate::records::topics;

#[derive(Debug, Serialize)]
pub struct QueueStatement {
    pub id: String,
    pub topic_id: String,
    pub topic_title: String,
    pub topic_status: String,
    pub status: String,
    pub body: String,
    /// AI・運営者の判定の理由
    pub moderation_note: Option<String>,
    pub created_at: String,
    /// seed（最初の意見）/ citizen（市民の投稿）/ added（あとから足した議会の主張・運営者の意見）
    pub kind: &'static str,
    pub basis: Vec<String>,
    pub document: Option<QueueDocument>,
}

#[derive(Debug, Serialize)]
pub struct QueueDocument {
    pub title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
}

impl From<statements::QueueItem> for QueueStatement {
    fn from(item: statements::QueueItem) -> Self {
        let s = item.statement;
        Self {
            kind: match (item.seed, item.posted) {
                (true, _) => "seed",
                (false, true) => "citizen",
                (false, false) => "added",
            },
            id: s.id,
            topic_id: s.topic_id,
            topic_title: item.topic_title,
            topic_status: item.topic_status,
            status: s.status,
            body: s.body,
            moderation_note: s.moderation_note,
            created_at: s.created_at,
            basis: item.basis,
            document: item.document.map(|d| QueueDocument {
                title: d.title,
                held_on: d.held_on,
                url: d.url,
            }),
        }
    }
}

// ===== 意見 =====

/// GET /api/operator/municipalities/{slug}/statements?after=&limit= — 確認待ち（flagged）と判定待ち（pending）
pub async fn queue(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(page): Query<PageQuery>,
) -> Result<Json<Page<QueueStatement>>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let limit = page.limit(QUEUE_PER_PAGE);
    let mut items: Vec<QueueStatement> = statements::queue_in(
        &state.pool,
        &municipality.id,
        page.after.as_deref(),
        limit + 1,
    )
    .await?
    .into_iter()
    .map(Into::into)
    .collect();
    let more = items.len() as i64 > limit;
    items.truncate(limit as usize);
    Ok(Json(Page::new(items, more, |s: &QueueStatement| &s.id)))
}

/// PATCH .../statements/{statement_id} — 本文を直す（投票がある意見は直せない）
pub async fn edit_statement(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, statement_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<StatementBody>,
) -> Result<StatusCode, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    statement_in(&state.pool, &municipality, &statement_id).await?;
    topics::edit_statement(&state.pool, &statement_id, &body.body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST .../statements/{statement_id}/approve — 公開する
pub async fn approve(
    op: Operator,
    state: State<AppState>,
    path: Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    decide(op, state, path, true).await
}

/// POST .../statements/{statement_id}/reject — 非公開にする（下書きの意見を取り下げるのも同じ）
pub async fn reject(
    op: Operator,
    state: State<AppState>,
    path: Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    decide(op, state, path, false).await
}

async fn decide(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, statement_id)): Path<(String, String)>,
    approve: bool,
) -> Result<StatusCode, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    statement_in(&state.pool, &municipality, &statement_id).await?;
    moderation::decide(&state.pool, &statement_id, approve).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct Retried {
    /// 判定のあとの状態（判定できなければ pending のまま）
    pub status: &'static str,
}

/// POST .../statements/{statement_id}/retry — 判定待ち（pending）の意見を AI でもう一度判定する
pub async fn retry(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, statement_id)): Path<(String, String)>,
) -> Result<Json<Retried>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let topic_id = statement_in(&state.pool, &municipality, &statement_id).await?;
    if matches!(state.moderation, Moderation::AutoApprove) {
        // キーが無い環境では、pending を勝手に承認しない（retry_pending と同じ）
        return Err(ApiError::conflict(
            "llm_unavailable",
            "AI が使えない設定です（ANTHROPIC_API_KEY）。公開・非公開を選んでください",
        ));
    }
    let detail = statements::load(&state.pool, &statement_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if detail.statement.status != "pending" {
        return Err(ApiError::conflict(
            "not_pending",
            "AI でやり直せるのは判定待ち（pending）の意見だけです",
        ));
    }
    let verdict = moderation::retry_one(
        &state.pool,
        &state.moderation,
        &statement_id,
        &topic_id,
        &detail.statement.body,
    )
    .await?;
    Ok(Json(Retried {
        status: verdict.status().as_str(),
    }))
}
