// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の提言の API（ADR 0005）。中身は CLI と同じ `process::recommendation`・`records::recommendations`

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::Operator;
use crate::interface::api::AppState;
use crate::interface::api::error::{ApiError, ApiJson};
use crate::interface::api::municipalities::{self, Municipality};
use crate::process::moderation::Moderation;
use crate::process::recommendation::{self, Candidate, CreateError, TextError};
use crate::records::recommendations::{self, Detail, EditError, StatusError, Summary};
use crate::shared::status::Publication;

impl From<CreateError> for ApiError {
    fn from(err: CreateError) -> Self {
        match err {
            CreateError::Database(e) => e.into(),
            CreateError::Empty => ApiError::validation("empty", err.to_string()),
            CreateError::Ineligible(_) => ApiError::validation("ineligible", err.to_string()),
            CreateError::InvalidChild(_) => ApiError::validation("invalid_child", err.to_string()),
        }
    }
}

impl From<StatusError> for ApiError {
    fn from(err: StatusError) -> Self {
        match err {
            StatusError::NotFound => ApiError::NotFound,
            StatusError::NoTitle => ApiError::validation("no_title", err.to_string()),
            StatusError::NoItems => ApiError::validation("no_items", err.to_string()),
            StatusError::WrongState => ApiError::conflict("wrong_state", err.to_string()),
        }
    }
}

impl From<EditError> for ApiError {
    fn from(err: EditError) -> Self {
        match err {
            EditError::NotDraft => ApiError::conflict("not_draft", err.to_string()),
            EditError::TitleTooLong | EditError::PrefaceTooLong => {
                ApiError::validation("too_long", err.to_string())
            }
        }
    }
}

/// その自治体の提言（別の自治体のものは 404）
async fn in_municipality(
    state: &AppState,
    municipality: &Municipality,
    id: &str,
) -> Result<Detail, ApiError> {
    recommendations::show(&state.pool, id)
        .await?
        .filter(|d| d.municipality_id == municipality.id)
        .ok_or(ApiError::NotFound)
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// draft（既定）/ published / archived
    pub status: Option<String>,
}

/// GET .../{slug}/recommendations?status= — 提言の一覧
pub async fn list(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Summary>>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let status = match query.status.as_deref() {
        None => Publication::Draft,
        Some(s) => Publication::ALL
            .into_iter()
            .find(|p| p.as_str() == s)
            .ok_or_else(|| {
                ApiError::validation("invalid_status", "status は draft / published / archived")
            })?,
    };
    Ok(Json(
        recommendations::list(&state.pool, &municipality.id, status).await?,
    ))
}

#[derive(Debug, Serialize)]
pub struct Candidates {
    /// 提言にできるトピック（締め切った順）
    pub topics: Vec<Candidate>,
    /// まとまりの候補（同じ分野タグを持ち、似ているトピックの id）
    pub groups: Vec<Vec<String>>,
    /// 子にできる公開済みの提言
    pub recommendations: Vec<Summary>,
}

/// GET .../{slug}/recommendation-candidates — 提言にできるトピックと、まとまりの候補
pub async fn candidates(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Candidates>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let (topics, groups) = recommendation::candidates(&state.pool, &municipality.id).await?;
    let published =
        recommendations::list(&state.pool, &municipality.id, Publication::Published).await?;
    Ok(Json(Candidates {
        topics,
        groups,
        recommendations: published,
    }))
}

#[derive(Debug, Deserialize)]
pub struct NewRecommendation {
    #[serde(default)]
    pub topic_ids: Vec<String>,
    #[serde(default)]
    pub child_ids: Vec<String>,
}

/// AI の見出し・前書きの下書きの結果（失敗しても提言はそのまま。手で書ける）
#[derive(Debug, Serialize)]
pub struct WithText {
    #[serde(flatten)]
    pub recommendation: Detail,
    /// AI に頼めなかった・使わなかった理由（API キーが無いときは null）
    pub text_error: Option<String>,
}

async fn write_text(state: &AppState, id: &str) -> Option<String> {
    let Moderation::Llm { llm, model } = &state.moderation else {
        return None;
    };
    match recommendation::draft_text(&state.pool, llm, model, id).await {
        Ok(_) => None,
        Err(TextError::Database(e)) => {
            tracing::warn!(error = %e, "recommendation text");
            Some("見出しと前書きの下書きを作れませんでした".into())
        }
        Err(e) => Some(e.to_string()),
    }
}

/// POST .../{slug}/recommendations — 下書きを作る（API キーがあれば見出しと前書きも AI が下書き）
pub async fn create(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    ApiJson(body): ApiJson<NewRecommendation>,
) -> Result<(StatusCode, Json<WithText>), ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let id = recommendation::create(
        &state.pool,
        &municipality.id,
        &body.topic_ids,
        &body.child_ids,
        state.divided_threshold,
    )
    .await?;
    let text_error = write_text(&state, &id).await;
    Ok((
        StatusCode::CREATED,
        Json(WithText {
            recommendation: in_municipality(&state, &municipality, &id).await?,
            text_error,
        }),
    ))
}

/// GET .../{slug}/recommendations/{id}
pub async fn show(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<Json<Detail>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    Ok(Json(in_municipality(&state, &municipality, &id).await?))
}

#[derive(Debug, Deserialize)]
pub struct EditText {
    pub title: Option<String>,
    pub preface: Option<String>,
}

/// PATCH .../{slug}/recommendations/{id} — 下書きの見出し・前書きを直す
pub async fn edit(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    ApiJson(body): ApiJson<EditText>,
) -> Result<Json<Detail>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    in_municipality(&state, &municipality, &id).await?;
    recommendations::edit(
        &state.pool,
        &id,
        body.title.as_deref(),
        body.preface.as_deref(),
    )
    .await??;
    Ok(Json(in_municipality(&state, &municipality, &id).await?))
}

/// POST .../{slug}/recommendations/{id}/text — 見出しと前書きを AI に書き直してもらう（API キーが要る）
pub async fn rewrite(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<Json<WithText>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    in_municipality(&state, &municipality, &id).await?;
    let text_error = match state.moderation {
        Moderation::AutoApprove => Some("API キーが無いので、AI には頼めません".to_owned()),
        Moderation::Llm { .. } => write_text(&state, &id).await,
    };
    Ok(Json(WithText {
        recommendation: in_municipality(&state, &municipality, &id).await?,
        text_error,
    }))
}

async fn set_status(
    state: AppState,
    slug: String,
    id: String,
    status: Publication,
) -> Result<Json<Detail>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    in_municipality(&state, &municipality, &id).await?;
    recommendations::set_status(&state.pool, &id, status).await??;
    Ok(Json(in_municipality(&state, &municipality, &id).await?))
}

/// POST .../{slug}/recommendations/{id}/publish — 公開する（下書きだけ。見出しと項目が要る）
pub async fn publish(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<Json<Detail>, ApiError> {
    set_status(state, slug, id, Publication::Published).await
}

/// POST .../{slug}/recommendations/{id}/archive — 取り下げる（公開済みだけ）
pub async fn archive(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<Json<Detail>, ApiError> {
    set_status(state, slug, id, Publication::Archived).await
}

/// DELETE .../{slug}/recommendations/{id} — 下書きを消す
pub async fn remove(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    in_municipality(&state, &municipality, &id).await?;
    if !recommendations::remove_draft(&state.pool, &id).await? {
        return Err(ApiError::conflict(
            "not_draft",
            "消せるのは下書きだけです（公開したものは取り下げる）",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct SecondRound {
    /// 作った第2回のトピックの下書きの ID
    pub topic_id: String,
}

/// POST .../{slug}/recommendations/{id}/second-round — 第2回のトピックの下書きを作る（公開済みの提言だけ）
pub async fn second_round(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<SecondRound>), ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    in_municipality(&state, &municipality, &id).await?;
    let topic_id = recommendations::second_round(&state.pool, &id)
        .await?
        .map_err(|e| ApiError::conflict("not_published", e.to_string()))?;
    Ok((StatusCode::CREATED, Json(SecondRound { topic_id })))
}
