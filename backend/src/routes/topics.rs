// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::AppState;
use crate::error::ApiError;
use crate::participant::MaybeParticipant;
use crate::routes::municipalities;

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

/// トピックへの参加状態。承認済みの意見だけで数える（意見が増えれば completed は in_progress に戻る）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Participation {
    /// まだ1件も投票していない
    None,
    /// 投票したが、未投票の意見が残っている
    InProgress,
    /// すべての意見に投票した
    Completed,
}

impl Participation {
    fn from_counts(voted: i64, total: i64) -> Self {
        if voted == 0 {
            Self::None
        } else if voted >= total {
            Self::Completed
        } else {
            Self::InProgress
        }
    }
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

/// 参加者がトピックごとに投票した、承認済みの意見の数
async fn voted_counts(
    pool: &SqlitePool,
    municipality_id: &str,
    user_id: Option<&str>,
) -> Result<HashMap<String, i64>, ApiError> {
    let Some(user_id) = user_id else {
        return Ok(HashMap::new());
    };
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT s.topic_id, count(*) FROM votes v
         JOIN statements s ON s.id = v.statement_id
         JOIN topics t ON t.id = s.topic_id
         WHERE v.user_id = $1 AND s.status = 'approved' AND t.municipality_id = $2
         GROUP BY s.topic_id",
    )
    .bind(user_id)
    .bind(municipality_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// 参加者がトピックで投票した承認済みの意見の数と、承認済みの意見の総数
pub async fn progress(
    pool: &SqlitePool,
    topic_id: &str,
    user_id: Option<&str>,
) -> Result<(i64, i64), ApiError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements WHERE topic_id = $1 AND status = 'approved'",
    )
    .bind(topic_id)
    .fetch_one(pool)
    .await?;
    let voted: i64 = match user_id {
        Some(uid) => {
            sqlx::query_scalar(
                "SELECT count(*) FROM votes v
                 JOIN statements s ON s.id = v.statement_id
                 WHERE s.topic_id = $1 AND s.status = 'approved' AND v.user_id = $2",
            )
            .bind(topic_id)
            .bind(uid)
            .fetch_one(pool)
            .await?
        }
        None => 0,
    };
    Ok((voted, total))
}

fn into_topic(row: TopicRow, tags: Vec<Tag>, voted_count: i64) -> Topic {
    Topic {
        participation: Participation::from_counts(voted_count, row.statement_count),
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

/// トピックが指定の自治体に属し、公開済みか確かめる。別の自治体のトピックや下書きなら 404。
/// 投票・投稿・次の意見・分析は、すべてここを通る
pub async fn ensure_in_municipality(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
) -> Result<(), ApiError> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT id FROM topics WHERE id = $1 AND municipality_id = $2 AND status = 'published'",
    )
    .bind(topic_id)
    .bind(municipality_id)
    .fetch_optional(pool)
    .await?;
    found.map(|_| ()).ok_or(ApiError::NotFound)
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
    let voted = voted_counts(&state.pool, &municipality.id, user_id.as_deref()).await?;

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
) -> Result<Json<Topic>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;

    let row: TopicRow = sqlx::query_as(&format!("{SELECT_TOPIC} AND t.id = $2"))
        .bind(&municipality.id)
        .bind(&topic_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    let tags = tags_by_topic(&state.pool, &municipality.id)
        .await?
        .remove(&row.id)
        .unwrap_or_default();
    let voted_count = voted_counts(&state.pool, &municipality.id, user_id.as_deref())
        .await?
        .get(&row.id)
        .copied()
        .unwrap_or(0);
    Ok(Json(into_topic(row, tags, voted_count)))
}
