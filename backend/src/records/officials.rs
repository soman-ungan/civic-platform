// SPDX-License-Identifier: AGPL-3.0-only

//! 議員の記録（ADR 0008）: 公人（首長・議員）と、その人の発言が出典になった公開済みのトピック。
//! 対応付けは会議録の取り込みで機械的に入れた `topic_sources.official_id` だけを使う（AI に推測させない）。
//! 順位・評価・市民の投票の結果は扱わない

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct Official {
    pub id: String,
    pub name: String,
    /// mayor / councilor
    pub role: String,
    pub term_start: String,
    pub term_end: Option<String>,
    /// その人の発言が出典になった、公開済みのトピックの数
    pub topic_count: i64,
}

const SELECT_OFFICIAL: &str = "
    SELECT o.id, o.name, o.role, o.term_start, o.term_end,
        (SELECT count(DISTINCT ts.topic_id) FROM topic_sources ts
           JOIN topics t ON t.id = ts.topic_id
          WHERE ts.official_id = o.id AND t.status = 'published') AS topic_count
    FROM public_officials o
    WHERE o.municipality_id = $1";

/// 並び: 首長が先、議員は読みの順（数では並べない）
const ORDER_OFFICIALS: &str = "
    ORDER BY CASE o.role WHEN 'mayor' THEN 0 ELSE 1 END, COALESCE(o.name_kana, o.name), o.id";

/// 一覧: 現職と、任期の終わった人のうち記録のある人
pub async fn list(pool: &SqlitePool, municipality_id: &str) -> Result<Vec<Official>, sqlx::Error> {
    let rows: Vec<Official> = sqlx::query_as(&format!("{SELECT_OFFICIAL} {ORDER_OFFICIALS}"))
        .bind(municipality_id)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .filter(|o| o.term_end.is_none() || o.topic_count > 0)
        .collect())
}

/// 1人（その自治体の人だけ）
pub async fn find(
    pool: &SqlitePool,
    municipality_id: &str,
    official_id: &str,
) -> Result<Option<Official>, sqlx::Error> {
    sqlx::query_as(&format!("{SELECT_OFFICIAL} AND o.id = $2"))
        .bind(municipality_id)
        .bind(official_id)
        .fetch_optional(pool)
        .await
}

/// その人の発言の引用（公開済みのトピックの出典だけ）。新しい会議の順、トピックの中では出典の順
#[derive(Debug, sqlx::FromRow)]
pub struct Quote {
    pub topic_id: String,
    pub topic_title: String,
    pub closes_at: Option<String>,
    pub quote: String,
    pub document_title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
}

pub async fn quotes(pool: &SqlitePool, official_id: &str) -> Result<Vec<Quote>, sqlx::Error> {
    sqlx::query_as(
        "SELECT t.id AS topic_id, t.title AS topic_title, t.closes_at,
                ts.quote, d.title AS document_title, d.held_on, d.url
         FROM topic_sources ts
         JOIN topics t ON t.id = ts.topic_id
         JOIN documents d ON d.id = ts.document_id
         WHERE ts.official_id = $1 AND t.status = 'published'
         ORDER BY d.held_on DESC, t.id, ts.position",
    )
    .bind(official_id)
    .fetch_all(pool)
    .await
}

/// その人の記録に出るトピックのタグ
#[derive(Debug, sqlx::FromRow)]
pub struct TopicTag {
    pub topic_id: String,
    pub id: String,
    pub kind: String,
    pub name: String,
}

pub async fn topic_tags(
    pool: &SqlitePool,
    official_id: &str,
) -> Result<Vec<TopicTag>, sqlx::Error> {
    sqlx::query_as(
        "SELECT DISTINCT tt.topic_id, g.id, g.kind, g.name
         FROM topic_tags tt
         JOIN tags g ON g.id = tt.tag_id
         JOIN topic_sources ts ON ts.topic_id = tt.topic_id
         JOIN topics t ON t.id = tt.topic_id
         WHERE ts.official_id = $1 AND t.status = 'published'
         ORDER BY g.kind, g.name",
    )
    .bind(official_id)
    .fetch_all(pool)
    .await
}

/// 取り込んだ会議録の範囲（最初と最後の会議の日）。記録が無いのは「発言していない」のではなく
/// 「取り込んだ範囲に載っていない」だけだと、画面で添えるため
pub async fn coverage(
    pool: &SqlitePool,
    municipality_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT min(held_on), max(held_on) FROM documents
         WHERE municipality_id = $1 AND kind = 'minutes' AND held_on IS NOT NULL",
    )
    .bind(municipality_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0.zip(row.1))
}
