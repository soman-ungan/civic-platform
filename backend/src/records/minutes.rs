// SPDX-License-Identifier: AGPL-3.0-only

//! 取り込んだ会議録と、そこから作ったトピック（CLI の list / show minutes）。
//! 取り込みは `ingest::minutes`。トピックの確認と公開は `records::topics`

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct Summary {
    /// `documents.id`
    pub id: String,
    pub slug: String,
    pub title: String,
    pub held_on: Option<String>,
    /// AI の出力の作り方（api / manual）
    pub extraction: Option<String>,
    /// 作ったトピックの数
    pub topics: i64,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Summary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT d.id, m.slug, d.title, d.held_on, d.extraction,
                (SELECT COUNT(DISTINCT ts.topic_id) FROM topic_sources ts WHERE ts.document_id = d.id) AS topics
         FROM documents d JOIN municipalities m ON m.id = d.municipality_id
         WHERE d.kind = 'minutes' ORDER BY m.slug, d.held_on, d.title",
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct Document {
    pub title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
    pub extraction: Option<String>,
    pub imported_at: String,
}

/// 会議録から作ったトピック
#[derive(Debug, sqlx::FromRow)]
pub struct TopicLine {
    pub id: String,
    pub status: String,
    pub title: String,
}

/// 1つの会議録と、そこから作ったトピック。無ければ None
pub async fn load(
    pool: &SqlitePool,
    document_id: &str,
) -> Result<Option<(Document, Vec<TopicLine>)>, sqlx::Error> {
    let doc: Option<Document> = sqlx::query_as(
        "SELECT title, held_on, url, extraction, imported_at FROM documents
         WHERE id = $1 AND kind = 'minutes'",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    let Some(doc) = doc else {
        return Ok(None);
    };
    let topics = sqlx::query_as(
        "SELECT DISTINCT t.id, t.status, t.title FROM topics t
         JOIN topic_sources ts ON ts.topic_id = t.id
         WHERE ts.document_id = $1 ORDER BY t.id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(Some((doc, topics)))
}
