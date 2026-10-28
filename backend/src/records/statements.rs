// SPDX-License-Identifier: AGPL-3.0-only

//! 意見の一覧と1件（CLI の list / show statement）。投稿者が誰かは、運営のコマンドでも返さない。
//! 意見の判定（承認・非公開）は `process::moderation`

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct Statement {
    pub id: String,
    pub topic_id: String,
    pub status: String,
    pub body: String,
    pub moderation_note: Option<String>,
    pub created_at: String,
}

/// 確認待ち（flagged）と判定待ち（pending）の意見（古い順）
pub async fn queue(pool: &SqlitePool) -> Result<Vec<Statement>, sqlx::Error> {
    list(pool, &["flagged".into(), "pending".into()], None, None).await
}

/// 状態・トピック・自治体（slug）で絞った意見（古い順）
pub async fn list(
    pool: &SqlitePool,
    statuses: &[String],
    topic_id: Option<&str>,
    slug: Option<&str>,
) -> Result<Vec<Statement>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        statement: Statement,
        slug: String,
    }
    // 絞り込みは手元で行う（両方の DB で動く SQL にするため）
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id, s.topic_id, s.status, s.body, s.moderation_note, s.created_at, m.slug
         FROM statements s JOIN topics t ON t.id = s.topic_id
         JOIN municipalities m ON m.id = t.municipality_id
         ORDER BY s.created_at",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|r| statuses.contains(&r.statement.status))
        .filter(|r| topic_id.is_none_or(|t| t == r.statement.topic_id))
        .filter(|r| slug.is_none_or(|m| m == r.slug))
        .map(|r| r.statement)
        .collect())
}

/// 1件の意見と投票の数
#[derive(Debug)]
pub struct Detail {
    pub statement: Statement,
    /// 最初の意見か（議会の発言から作ったもの・下書きのときに運営者が足したもの）
    pub seed: bool,
    /// 投稿者がいるか（市民の投稿か。いなければ、議会の主張か運営者が足したもの）
    pub posted: bool,
    pub agree: i64,
    pub disagree: i64,
    pub pass: i64,
}

pub async fn load(pool: &SqlitePool, id: &str) -> Result<Option<Detail>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        statement: Statement,
        seed: bool,
        posted: bool,
    }
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, topic_id, status, body, moderation_note, created_at, is_seed AS seed,
                user_id IS NOT NULL AS posted
         FROM statements WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let votes: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT vote_value, COUNT(*) FROM votes WHERE statement_id = $1 GROUP BY vote_value",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let count = |v: i64| votes.iter().find(|(x, _)| *x == v).map_or(0, |(_, n)| *n);
    Ok(Some(Detail {
        statement: row.statement,
        seed: row.seed,
        posted: row.posted,
        agree: count(1),
        disagree: count(-1),
        pass: count(0),
    }))
}
