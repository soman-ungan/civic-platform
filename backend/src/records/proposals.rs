// SPDX-License-Identifier: AGPL-3.0-only

//! 市民からのトピックの提案（ADR 0005。docs/plans/2026-10-29-phase6d-citizen-proposals.md）。
//! 提案は下書き（`status = 'draft'`・`source_type = 'citizen'`）として保存し、運営者が確かめてから公開する
//! （公開・見送りは管理画面の下書きと同じ `topics::set_status`）。
//! 提案した人（`topics.proposed_by`）は、本人の「自分の提案」の一覧のほかには出さない（匿名性）

use sqlx::SqlitePool;

use crate::shared::util::{new_id, now_iso};

/// 提案を保存する。意見は提案した人の意見として承認済みの最初の意見にする（運営者が下書きで確かめる）。
/// 入力の確かめ（文字数・公人の氏名）は呼ぶ側で済ませておく。戻り値はトピックの ID
pub async fn create(
    pool: &SqlitePool,
    municipality_id: &str,
    user_id: &str,
    title: &str,
    description: &str,
    statements: &[&str],
) -> Result<String, sqlx::Error> {
    let topic_id = new_id();
    let now = now_iso();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO topics
            (id, municipality_id, title, description, source_type, status, proposed_by, created_at)
         VALUES ($1, $2, $3, $4, 'citizen', 'draft', $5, $6)",
    )
    .bind(&topic_id)
    .bind(municipality_id)
    .bind(title)
    .bind(description)
    .bind(user_id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    for body in statements {
        sqlx::query(
            "INSERT INTO statements (id, topic_id, user_id, body, status, created_at, is_seed)
             VALUES ($1, $2, $3, $4, 'approved', $5, TRUE)",
        )
        .bind(new_id())
        .bind(&topic_id)
        .bind(user_id)
        .bind(body)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(topic_id)
}

/// 「自分の提案」の1件
#[derive(Debug, sqlx::FromRow)]
pub struct MyProposal {
    pub id: String,
    pub title: String,
    pub description: String,
    /// draft / published / archived
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
}

/// その人がこの自治体に出した提案（新しい順）
pub async fn mine(
    pool: &SqlitePool,
    municipality_id: &str,
    user_id: &str,
) -> Result<Vec<MyProposal>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, description, status, published_at, created_at FROM topics
         WHERE proposed_by = $1 AND municipality_id = $2
         ORDER BY created_at DESC, id DESC",
    )
    .bind(user_id)
    .bind(municipality_id)
    .fetch_all(pool)
    .await
}

/// 公開したときに、提案した人の賛成の票を、その人の承認済みの意見に入れる（投稿したら賛成とみなす。ADR 0005）。
/// 提出のときに入れないのは、投票のある意見は運営者が直せないため。何度呼んでも同じ
pub async fn add_proposer_votes(pool: &SqlitePool, topic_id: &str) -> Result<(), sqlx::Error> {
    let statements: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.id, s.user_id FROM statements s
         JOIN topics t ON t.id = s.topic_id
         WHERE s.topic_id = $1 AND s.status = 'approved' AND s.user_id = t.proposed_by",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    for (statement_id, user_id) in statements {
        sqlx::query(
            "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at)
             VALUES ($1, $2, $3, 1, $4)
             ON CONFLICT (user_id, statement_id) DO NOTHING",
        )
        .bind(new_id())
        .bind(&user_id)
        .bind(&statement_id)
        .bind(now_iso())
        .execute(pool)
        .await?;
    }
    Ok(())
}
