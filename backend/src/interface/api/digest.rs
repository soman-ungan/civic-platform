// SPDX-License-Identifier: AGPL-3.0-only

//! トピックマップと市民のまとめの API（ADR 0005「市民のまとめ」）。
//! 締め切ったトピックだけを扱い、保存はしない（締め切ったトピックは変わらないので、そのつど計算する）。
//! 参加者 ID は返さない（関わり方の人数は二次秘匿済み）

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use super::AppState;
use super::error::ApiError;
use super::municipalities;
use crate::process::digest::{self, Digest};
use crate::process::topic_map::{self, TopicMap};

#[derive(Debug, Deserialize)]
pub struct MapQuery {
    pub tag: Option<String>,
}

/// GET /api/municipalities/{slug}/topic-map?tag= — 締め切ったトピックの地図（タグで絞る。無ければタグの一覧だけ）
pub async fn map(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<MapQuery>,
) -> Result<Json<TopicMap>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    Ok(Json(
        topic_map::topic_map(&state.pool, &municipality.id, query.tag.as_deref()).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct DigestQuery {
    /// 起点のトピック
    pub anchor: String,
    /// 起点の近傍から選んだトピック（カンマ区切り。起点は含めなくてよい）
    #[serde(default)]
    pub topics: String,
}

/// GET /api/municipalities/{slug}/digest?anchor=&topics= — 選んだトピックの合意点・意見が分かれた点・代表的な意見
pub async fn digest(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<DigestQuery>,
) -> Result<Json<Digest>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let topics: Vec<String> = query
        .topics
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    digest::digest(&state.pool, &municipality.id, &query.anchor, &topics)
        .await?
        .map(Json)
        .map_err(|e| ApiError::validation("invalid_selection", e.to_string()))
}
