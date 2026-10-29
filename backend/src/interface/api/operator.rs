// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の API（`/api/operator/...`。フェーズ6c。docs/plans/2026-10-29-phase6c-operator-console.md）。
//!
//! - 運営者（`users.is_operator`）だけが使える。ほかの人には、あること自体を見せないよう 404 を返す
//! - 自治体で絞り込む。トピック・意見がパスの自治体に属していなければ 404
//! - 中身は CLI と同じ `records`・`process` の関数を使う（ロジックを写さない）。判定の基準は ADR 0007
//! - 投稿者が誰かは、運営者にも返さない（市民の投稿か、議会の主張・運営者の意見かだけ）

use axum::Json;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::AppState;
use super::error::{ApiError, ApiJson};
use super::municipalities::{self, Municipality};
use super::participant::MaybeParticipant;
use crate::process::moderation::{self, Moderation};
use crate::process::progress;
use crate::records::topics::{self, DraftDetail, EditError};
use crate::records::{accounts, statements, tags};
use crate::shared::status::Publication;
use crate::shared::util::now_iso;

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
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub closes_at: Option<String>,
    pub closed: bool,
    pub sources: Vec<Source>,
    pub statements: Vec<TopicStatement>,
    pub tags: Vec<TopicTag>,
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
            id: d.id,
            status: d.status,
            title: d.title,
            description: d.description,
            created_at: d.created_at,
            published_at: d.published_at,
            closes_at: d.closes_at,
        }
    }
}

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

// ===== 分けて読む =====

/// 1回に返すトピックの数（出典・意見・タグを含むので少なめ）
const TOPICS_PER_PAGE: i64 = 10;
/// 1回に返す確認待ちの意見の数
const QUEUE_PER_PAGE: i64 = 20;
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
async fn topic_in(
    pool: &SqlitePool,
    municipality: &Municipality,
    topic_id: &str,
) -> Result<DraftDetail, ApiError> {
    topics::show(pool, topic_id)
        .await?
        .filter(|t| t.municipality_id == municipality.id)
        .ok_or(ApiError::NotFound)
}

/// 意見がこの自治体のものか確かめ、トピックの ID を返す
async fn statement_in(
    pool: &SqlitePool,
    municipality: &Municipality,
    statement_id: &str,
) -> Result<String, ApiError> {
    match statements::owner(pool, statement_id).await? {
        Some((topic_id, m)) if m == municipality.id => Ok(topic_id),
        _ => Err(ApiError::NotFound),
    }
}

async fn reload(pool: &SqlitePool, topic_id: &str) -> Result<Json<OperatorTopic>, ApiError> {
    let detail = topics::show(pool, topic_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(detail.into()))
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
    Ok(Json(
        topic_in(&state.pool, &municipality, &topic_id)
            .await?
            .into(),
    ))
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
