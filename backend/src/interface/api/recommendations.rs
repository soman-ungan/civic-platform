// SPDX-License-Identifier: AGPL-3.0-only

//! 提言の API（公開済みだけ。ADR 0005）。中身は作ったときの原文と数字の写し（`records::recommendations`）。
//! 関わり方の内訳は二次秘匿の済んだもの。参加者 ID は持っていない

use axum::Json;
use axum::extract::{Path, State};

use super::AppState;
use super::error::ApiError;
use super::municipalities;
use crate::records::recommendations::{self, Detail, Summary};
use crate::shared::status::Publication;

/// GET /api/municipalities/{slug}/recommendations — 公開済みの提言（公開の新しい順）
pub async fn list(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<Summary>>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    Ok(Json(
        recommendations::list(&state.pool, &municipality.id, Publication::Published).await?,
    ))
}

/// GET /api/municipalities/{slug}/recommendations/{id} — 公開済みの提言1件（下書き・取り下げ・別の自治体は 404）
pub async fn detail(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<Json<Detail>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let detail = recommendations::show(&state.pool, &id)
        .await?
        .filter(|d| d.municipality_id == municipality.id && d.status == "published")
        .ok_or(ApiError::NotFound)?;
    Ok(Json(detail))
}
