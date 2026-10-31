// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の API: 自動公開（公開待ちと公開の時刻。docs/plans/2026-10-30-timeline-flow.md）

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::topics::{OperatorTopic, reload};
use super::{Operator, topic_in};
use crate::interface::api::AppState;
use crate::interface::api::error::{ApiError, ApiJson};
use crate::interface::api::municipalities::{self, Municipality};
use crate::process::publishing;
use crate::records::publishing::{self as queue_records, SlotError};

// ===== 自動公開（docs/plans/2026-10-30-timeline-flow.md） =====

impl From<SlotError> for ApiError {
    fn from(err: SlotError) -> Self {
        match err {
            SlotError::NotFound(_) => Self::NotFound,
            SlotError::Invalid(message) => Self::validation("invalid_slot", message),
            SlotError::Database(err) => Self::Database(err),
        }
    }
}

/// POST .../topics/{topic_id}/queue — 公開待ちに入れる（下書きだけ。ほかは 409）
pub async fn enqueue(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    topic_in(&state.pool, &municipality, &topic_id).await?;
    if !queue_records::enqueue(&state.pool, Some(&municipality.id), &topic_id).await? {
        return Err(ApiError::conflict(
            "not_draft",
            "公開待ちに入れられるのは下書きだけです",
        ));
    }
    reload(&state.pool, &topic_id).await
}

/// DELETE .../topics/{topic_id}/queue — 公開待ちから外す（下書きのまま）
pub async fn dequeue(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    topic_in(&state.pool, &municipality, &topic_id).await?;
    queue_records::dequeue(&state.pool, Some(&municipality.id), &topic_id).await?;
    reload(&state.pool, &topic_id).await
}

#[derive(Debug, Serialize)]
pub struct QueuedTopic {
    pub id: String,
    pub title: String,
    pub source_type: String,
    pub queued_at: String,
    /// 公開の予定（ISO8601）。時刻が1つも無ければ null
    pub planned_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublishSlot {
    pub id: String,
    /// 日本時間の HH:MM
    pub time_of_day: String,
}

/// 自動公開の全体: 公開待ち（公開する順）と、公開の時刻
#[derive(Debug, Serialize)]
pub struct Publishing {
    pub queue: Vec<QueuedTopic>,
    pub slots: Vec<PublishSlot>,
}

async fn publishing_view(
    pool: &SqlitePool,
    municipality: &Municipality,
) -> Result<Json<Publishing>, ApiError> {
    let queue = queue_records::queue(pool, &municipality.id).await?;
    let slots = queue_records::slots(pool, Some(&municipality.id)).await?;
    let planned = publishing::planned(&slots, queue.len(), chrono::Utc::now());
    Ok(Json(Publishing {
        queue: queue
            .into_iter()
            .enumerate()
            .map(|(i, q)| QueuedTopic {
                id: q.id,
                title: q.title,
                source_type: q.source_type,
                queued_at: q.queued_at,
                planned_at: planned.get(i).cloned(),
            })
            .collect(),
        slots: slots
            .into_iter()
            .map(|s| PublishSlot {
                id: s.id,
                time_of_day: s.time_of_day,
            })
            .collect(),
    }))
}

/// GET .../publishing — 公開待ちと公開の時刻
pub async fn publishing(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Publishing>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    publishing_view(&state.pool, &municipality).await
}

#[derive(Debug, Deserialize)]
pub struct SlotInput {
    /// 日本時間の HH:MM
    pub time_of_day: String,
}

/// POST .../publish-slots — 公開の時刻を足す（今日の時刻を過ぎていれば明日から）
pub async fn add_slot(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    ApiJson(body): ApiJson<SlotInput>,
) -> Result<Json<Publishing>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    publishing::add_slot(
        &state.pool,
        &municipality.id,
        &body.time_of_day,
        chrono::Utc::now(),
    )
    .await?;
    publishing_view(&state.pool, &municipality).await
}

/// PATCH .../publish-slots/{slot_id} — 公開の時刻を変える
pub async fn update_slot(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, slot_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<SlotInput>,
) -> Result<Json<Publishing>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    publishing::update_slot(
        &state.pool,
        &municipality.id,
        &slot_id,
        &body.time_of_day,
        chrono::Utc::now(),
    )
    .await?;
    publishing_view(&state.pool, &municipality).await
}

/// DELETE .../publish-slots/{slot_id} — 公開の時刻を消す
pub async fn delete_slot(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, slot_id)): Path<(String, String)>,
) -> Result<Json<Publishing>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    if !queue_records::remove_slot(&state.pool, Some(&municipality.id), &slot_id).await? {
        return Err(ApiError::NotFound);
    }
    publishing_view(&state.pool, &municipality).await
}
