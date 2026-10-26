// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::AppState;
use crate::error::ApiError;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Municipality {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub prefecture_name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct District {
    pub id: String,
    pub name: String,
    pub name_kana: String,
}

#[derive(Debug, Serialize)]
pub struct MunicipalityDetail {
    #[serde(flatten)]
    pub municipality: Municipality,
    pub districts: Vec<District>,
}

const SELECT_MUNICIPALITY: &str = "
    SELECT m.id, m.slug, m.name, p.name AS prefecture_name
    FROM municipalities m
    JOIN prefectures p ON p.code = m.prefecture_code
    WHERE m.is_active = TRUE";

/// slug から公開中の自治体を引く。無効・存在しない場合は 404。
/// 自治体に属するデータを扱うハンドラは、必ずここで得た `id` で絞り込むこと。
pub async fn resolve(pool: &SqlitePool, slug: &str) -> Result<Municipality, ApiError> {
    sqlx::query_as(&format!("{SELECT_MUNICIPALITY} AND m.slug = $1"))
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

/// GET /api/municipalities — 公開中の自治体の一覧
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Municipality>>, ApiError> {
    let rows = sqlx::query_as(&format!("{SELECT_MUNICIPALITY} ORDER BY m.id"))
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows))
}

/// GET /api/municipalities/{slug} — 自治体と地区の一覧
pub async fn detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<MunicipalityDetail>, ApiError> {
    let municipality = resolve(&state.pool, &slug).await?;
    let districts = sqlx::query_as(
        "SELECT id, name, name_kana FROM districts WHERE municipality_id = $1 ORDER BY name_kana",
    )
    .bind(&municipality.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(MunicipalityDetail {
        municipality,
        districts,
    }))
}
