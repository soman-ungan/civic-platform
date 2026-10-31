// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の API: トピック（下書き・公開・期限・意見とタグ。似ているトピックの候補つき）

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{Operator, Page, PageQuery, TOPICS_PER_PAGE, topic_in};
use crate::interface::api::AppState;
use crate::interface::api::error::{ApiError, ApiJson};
use crate::interface::api::municipalities;
use crate::process::progress;
use crate::process::similarity::{self, SimilarTopic};
use crate::records::topics::{self, DraftDetail};
use crate::shared::status::Publication;
use crate::shared::util::now_iso;

// ===== 返す形 =====

#[derive(Debug, Serialize)]
pub struct Source {
    pub document_title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
    pub utterance_no: i64,
    pub speaker: String,
    pub quote: String,
}

#[derive(Debug, Serialize)]
pub struct TopicStatement {
    pub id: String,
    pub body: String,
    pub status: String,
    /// 根拠の発言（「発言12 〇〇議員」）。会議録から作った意見だけ
    pub basis: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TopicParent {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct TopicTag {
    pub id: String,
    pub name: String,
    /// ai / admin
    pub assigned_by: String,
}

/// 運営者が確かめるための、トピックの全体
#[derive(Debug, Serialize)]
pub struct OperatorTopic {
    pub id: String,
    pub status: String,
    /// assembly（会議録から）/ citizen（市民の提案。提案した人は返さない）
    pub source_type: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub published_at: Option<String>,
    /// 公開待ちに入れた日時（下書きだけ。NULL は入っていない）
    pub queued_at: Option<String>,
    pub closes_at: Option<String>,
    pub closed: bool,
    pub sources: Vec<Source>,
    pub statements: Vec<TopicStatement>,
    pub tags: Vec<TopicTag>,
    /// 市民のまとめから作ったトピックなら、元のトピック（`{id, title}`）
    pub parents: Vec<TopicParent>,
    /// 似ている同じ自治体のトピック（下書きだけ。同じ論点の重なりを確かめる候補。process::similarity）
    pub similar: Vec<SimilarTopic>,
}

impl From<DraftDetail> for OperatorTopic {
    fn from(mut d: DraftDetail) -> Self {
        Self {
            closed: progress::is_closed(d.closes_at.as_deref()),
            statements: d
                .statements
                .into_iter()
                .map(|(id, body, status)| TopicStatement {
                    basis: d.basis.remove(&id).unwrap_or_default(),
                    id,
                    body,
                    status,
                })
                .collect(),
            sources: d
                .sources
                .into_iter()
                .map(|s| Source {
                    document_title: s.document_title,
                    held_on: s.held_on,
                    url: s.url,
                    utterance_no: s.utterance_no,
                    speaker: s.speaker_label,
                    quote: s.quote,
                })
                .collect(),
            tags: d
                .tags
                .into_iter()
                .map(|(id, name, assigned_by)| TopicTag {
                    id,
                    name,
                    assigned_by,
                })
                .collect(),
            parents: d
                .parents
                .into_iter()
                .map(|(id, title)| TopicParent { id, title })
                .collect(),
            id: d.id,
            status: d.status,
            source_type: d.source_type,
            title: d.title,
            description: d.description,
            created_at: d.created_at,
            published_at: d.published_at,
            queued_at: d.queued_at,
            closes_at: d.closes_at,
            similar: Vec::new(),
        }
    }
}

/// 下書きに似ているトピックを、いくつまで見せるか
const SIMILAR_TOPICS_SHOWN: usize = 3;

/// 下書きに、似ているトピックの候補を付ける
async fn with_similar(
    pool: &SqlitePool,
    municipality_id: &str,
    items: &mut [OperatorTopic],
) -> Result<(), sqlx::Error> {
    let drafts: Vec<String> = items
        .iter()
        .filter(|t| t.status == "draft")
        .map(|t| t.id.clone())
        .collect();
    if drafts.is_empty() {
        return Ok(());
    }
    let mut similar =
        similarity::similar_topics(pool, municipality_id, &drafts, SIMILAR_TOPICS_SHOWN).await?;
    for t in items.iter_mut() {
        if let Some(v) = similar.remove(&t.id) {
            t.similar = v;
        }
    }
    Ok(())
}

/// 1件のトピックを読み直して返す（操作のあとの応答）
pub(super) async fn reload(
    pool: &SqlitePool,
    topic_id: &str,
) -> Result<Json<OperatorTopic>, ApiError> {
    let detail = topics::show(pool, topic_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(detail.into()))
}

// ===== トピック =====

#[derive(Debug, Deserialize)]
pub struct TopicsQuery {
    /// draft（既定）/ published / archived
    #[serde(default)]
    pub status: Option<String>,
    // `PageQuery` を flatten すると、クエリの文字列から数（limit）を読めないため、ここにも書く
    pub after: Option<String>,
    pub limit: Option<i64>,
}

fn publication(status: Option<&str>) -> Result<Publication, ApiError> {
    match status {
        None => Ok(Publication::Draft),
        Some(s) => Publication::ALL
            .into_iter()
            .find(|p| p.as_str() == s)
            .ok_or_else(|| {
                ApiError::validation("invalid_status", "status は draft / published / archived")
            }),
    }
}

/// GET /api/operator/municipalities/{slug}/topics?status=&after=&limit= — 下書きは古い順、ほかは新しい順
pub async fn list_topics(
    _: Operator,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<TopicsQuery>,
) -> Result<Json<Page<OperatorTopic>>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let status = publication(query.status.as_deref())?;
    let page = PageQuery {
        after: query.after,
        limit: query.limit,
    };
    let limit = page.limit(TOPICS_PER_PAGE);
    let ids = topics::ids_in(
        &state.pool,
        &municipality.id,
        status,
        page.after.as_deref(),
        limit + 1,
    )
    .await?;
    let mut items = Vec::new();
    for id in ids.iter().take(limit as usize) {
        if let Some(detail) = topics::show(&state.pool, id).await? {
            items.push(detail.into());
        }
    }
    with_similar(&state.pool, &municipality.id, &mut items).await?;
    Ok(Json(Page::new(
        items,
        ids.len() as i64 > limit,
        |t: &OperatorTopic| &t.id,
    )))
}

/// GET /api/operator/municipalities/{slug}/topics/{topic_id}
pub async fn topic(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let mut items = [OperatorTopic::from(
        topic_in(&state.pool, &municipality, &topic_id).await?,
    )];
    with_similar(&state.pool, &municipality.id, &mut items).await?;
    let [topic] = items;
    Ok(Json(topic))
}

#[derive(Debug, Deserialize)]
pub struct EditTopic {
    pub title: Option<String>,
    pub description: Option<String>,
}

/// PATCH /api/operator/municipalities/{slug}/topics/{topic_id} — タイトル・説明を直す
pub async fn edit_topic(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<EditTopic>,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    topic_in(&state.pool, &municipality, &topic_id).await?;
    topics::edit_topic(
        &state.pool,
        &topic_id,
        body.title.as_deref(),
        body.description.as_deref(),
    )
    .await?;
    reload(&state.pool, &topic_id).await
}

/// POST .../topics/{topic_id}/publish — 公開する（初めてなら期限が付く。ADR 0005）
pub async fn publish_topic(
    op: Operator,
    state: State<AppState>,
    path: Path<(String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    set_status(op, state, path, Publication::Published).await
}

/// POST .../topics/{topic_id}/archive — 取り下げる（公開をやめる）
pub async fn archive_topic(
    op: Operator,
    state: State<AppState>,
    path: Path<(String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    set_status(op, state, path, Publication::Archived).await
}

async fn set_status(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    status: Publication,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    topic_in(&state.pool, &municipality, &topic_id).await?;
    topics::set_status(
        &state.pool,
        &topic_id,
        status,
        &progress::default_closes_at(),
    )
    .await?;
    reload(&state.pool, &topic_id).await
}

#[derive(Debug, Deserialize)]
pub struct Deadline {
    /// 新しい期限（RFC 3339。例: 2026-12-01T23:59:59+09:00）。省くと今（すぐ締め切る）
    pub closes_at: Option<String>,
}

/// PUT .../topics/{topic_id}/closes-at — 期限を変える・すぐ締め切る。公開中のトピックだけ
pub async fn set_closes_at(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<Deadline>,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    let topic = topic_in(&state.pool, &municipality, &topic_id).await?;
    let closes_at = match body.closes_at.as_deref() {
        None => now_iso(),
        Some(input) => progress::normalize_closes_at(input).ok_or_else(|| {
            ApiError::validation(
                "invalid_closes_at",
                "期限は日時とタイムゾーンで指定してください（例: 2026-12-01T23:59:59+09:00）",
            )
        })?,
    };
    if !topics::set_closes_at(&state.pool, &topic.id, &closes_at).await? {
        return Err(ApiError::conflict(
            "not_published",
            "期限を変えられるのは公開中のトピックだけです",
        ));
    }
    reload(&state.pool, &topic_id).await
}

// ===== トピックの意見・タグ =====

#[derive(Debug, Deserialize)]
pub struct StatementBody {
    pub body: String,
}

/// POST .../topics/{topic_id}/statements — 意見を足す（運営者の意見として承認済み。下書きなら最初の意見）
pub async fn add_statement(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    ApiJson(body): ApiJson<StatementBody>,
) -> Result<(StatusCode, Json<OperatorTopic>), ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    topic_in(&state.pool, &municipality, &topic_id).await?;
    topics::add_statement(&state.pool, &topic_id, &body.body).await?;
    Ok((StatusCode::CREATED, reload(&state.pool, &topic_id).await?))
}

/// PUT / DELETE .../topics/{topic_id}/tags/{tag_id} — タグを付ける（運営者のタグとして）・外す
pub async fn put_tag(
    op: Operator,
    state: State<AppState>,
    path: Path<(String, String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    set_tag(op, state, path, true).await
}

pub async fn delete_tag(
    op: Operator,
    state: State<AppState>,
    path: Path<(String, String, String)>,
) -> Result<Json<OperatorTopic>, ApiError> {
    set_tag(op, state, path, false).await
}

async fn set_tag(
    _: Operator,
    State(state): State<AppState>,
    Path((slug, topic_id, tag_id)): Path<(String, String, String)>,
    add: bool,
) -> Result<Json<OperatorTopic>, ApiError> {
    let municipality = municipalities::resolve_any(&state.pool, &slug).await?;
    topic_in(&state.pool, &municipality, &topic_id).await?;
    topics::set_tag(&state.pool, &topic_id, &tag_id, add).await?;
    reload(&state.pool, &topic_id).await
}
