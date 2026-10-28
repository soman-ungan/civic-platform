// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::interface::api::AppState;
use crate::interface::api::error::ApiError;
use crate::interface::api::participant::MaybeParticipant;
use crate::interface::api::{finances, municipalities};
use crate::progress::{self, Participation, Progress};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct TopicRow {
    id: String,
    title: String,
    description: String,
    source_type: String,
    source_url: Option<String>,
    created_at: String,
    statement_count: i64,
    participant_count: i64,
}

#[derive(Debug, Serialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source_type: String,
    pub source_url: Option<String>,
    pub created_at: String,
    /// 承認済みの意見の数
    pub statement_count: i64,
    /// 1件以上投票した人の数
    pub participant_count: i64,
    pub tags: Vec<Tag>,
    /// Cookie の参加者の参加状態（Cookie が無ければ none）
    pub participation: Participation,
    /// Cookie の参加者が投票した、承認済みの意見の数
    pub voted_count: i64,
}

/// 詳細: トピックと、議会のトピックの出典
#[derive(Debug, Serialize)]
pub struct TopicDetail {
    #[serde(flatten)]
    pub topic: Topic,
    pub sources: Vec<TopicSource>,
    /// トピックの分野に関係する市のお金（決算カード）。対応する費目が無ければ null
    pub finances: Option<finances::TopicFinances>,
}

/// 出典（会議録の発言からの引用）。発言者は公人なら氏名つき、職員は職名だけ
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TopicSource {
    pub speaker: String,
    pub quote: String,
    pub document_title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TopicList {
    pub topics: Vec<Topic>,
    /// この自治体のトピックに付いているタグ（絞り込みのチップ用。絞り込み前の全件）
    pub tags: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub tag: Option<String>,
}

/// $1 = municipality_id。$2 以降は呼び出し側で追加する。公開済みのトピックだけ（下書きは運営者の確認待ち）
const SELECT_TOPIC: &str = "
    SELECT t.id, t.title, t.description, t.source_type, t.source_url, t.created_at,
        (SELECT count(*) FROM statements s
          WHERE s.topic_id = t.id AND s.status = 'approved') AS statement_count,
        (SELECT count(DISTINCT v.user_id) FROM votes v
          JOIN statements s ON s.id = v.statement_id
          WHERE s.topic_id = t.id) AS participant_count
    FROM topics t
    WHERE t.municipality_id = $1 AND t.status = 'published'";

#[derive(Debug, sqlx::FromRow)]
struct TopicTagRow {
    topic_id: String,
    #[sqlx(flatten)]
    tag: Tag,
}

/// 自治体内のトピックに付いたタグを topic_id ごとにまとめる
async fn tags_by_topic(
    pool: &SqlitePool,
    municipality_id: &str,
) -> Result<HashMap<String, Vec<Tag>>, ApiError> {
    let rows: Vec<TopicTagRow> = sqlx::query_as(
        "SELECT tt.topic_id, g.id, g.kind, g.name
         FROM topic_tags tt
         JOIN tags g ON g.id = tt.tag_id
         JOIN topics t ON t.id = tt.topic_id
         WHERE t.municipality_id = $1 AND t.status = 'published'
         ORDER BY g.kind, g.name",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<String, Vec<Tag>> = HashMap::new();
    for row in rows {
        map.entry(row.topic_id).or_default().push(row.tag);
    }
    Ok(map)
}

fn into_topic(row: TopicRow, tags: Vec<Tag>, voted_count: i64) -> Topic {
    Topic {
        participation: Progress {
            voted: voted_count,
            total: row.statement_count,
        }
        .participation(),
        voted_count,
        id: row.id,
        title: row.title,
        description: row.description,
        source_type: row.source_type,
        source_url: row.source_url,
        created_at: row.created_at,
        statement_count: row.statement_count,
        participant_count: row.participant_count,
        tags,
    }
}

/// GET /api/municipalities/{slug}/topics?tag={tag_id} — 新しい順
pub async fn list(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<TopicList>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;

    // PostgreSQL では型の決まらないパラメーターを IS NULL にできないため CAST する
    let rows: Vec<TopicRow> = sqlx::query_as(&format!(
        "{SELECT_TOPIC}
           AND (CAST($2 AS TEXT) IS NULL OR EXISTS (
                SELECT 1 FROM topic_tags tt WHERE tt.topic_id = t.id AND tt.tag_id = $2))
         ORDER BY t.created_at DESC, t.id DESC"
    ))
    .bind(&municipality.id)
    .bind(&query.tag)
    .fetch_all(&state.pool)
    .await?;

    let mut tags_map = tags_by_topic(&state.pool, &municipality.id).await?;
    let voted = progress::voted_counts(&state.pool, &municipality.id, user_id.as_deref()).await?;

    // チップ用: 重複を除いた全タグ（kind, name 順）
    let mut all_tags: Vec<Tag> = tags_map.values().flatten().cloned().collect();
    all_tags.sort_by(|a, b| (&a.kind, &a.name).cmp(&(&b.kind, &b.name)));
    all_tags.dedup_by(|a, b| a.id == b.id);

    let topics = rows
        .into_iter()
        .map(|row| {
            let tags = tags_map.remove(&row.id).unwrap_or_default();
            let voted_count = voted.get(&row.id).copied().unwrap_or(0);
            into_topic(row, tags, voted_count)
        })
        .collect();

    Ok(Json(TopicList {
        topics,
        tags: all_tags,
    }))
}

/// GET /api/municipalities/{slug}/topics/{topic_id}
pub async fn detail(
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<TopicDetail>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;

    let row: TopicRow = sqlx::query_as(&format!("{SELECT_TOPIC} AND t.id = $2"))
        .bind(&municipality.id)
        .bind(&topic_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    let tags = sqlx::query_as(
        "SELECT g.id, g.kind, g.name FROM topic_tags tt JOIN tags g ON g.id = tt.tag_id
         WHERE tt.topic_id = $1 ORDER BY g.kind, g.name",
    )
    .bind(&row.id)
    .fetch_all(&state.pool)
    .await?;
    let voted_count = progress::progress(&state.pool, &row.id, user_id.as_deref())
        .await?
        .voted;
    let sources = sqlx::query_as(
        "SELECT s.speaker_label AS speaker, s.quote, d.title AS document_title, d.held_on, d.url
         FROM topic_sources s JOIN documents d ON d.id = s.document_id
         WHERE s.topic_id = $1 ORDER BY s.position",
    )
    .bind(&row.id)
    .fetch_all(&state.pool)
    .await?;
    let finances = finances::for_topic(&state.pool, &municipality.id, &row.id).await?;
    Ok(Json(TopicDetail {
        topic: into_topic(row, tags, voted_count),
        sources,
        finances,
    }))
}
