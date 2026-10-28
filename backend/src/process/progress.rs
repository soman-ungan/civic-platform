// SPDX-License-Identifier: AGPL-3.0-only

//! 参加のルール: 参加者の進み具合（投票した意見の数）、参加の状態、分析を見られるか、
//! トピックに参加できるか（自治体・公開の状態）、このまちとの関わり方。
//! 投票・投稿・次の意見・分析・トピックの一覧は、すべてここのルールを使う（ルールを1か所で変えられるように）

use std::collections::HashMap;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use sqlx::SqlitePool;

/// このまちとの関わり方（ADR 0002。市民を分けてよいのは、これだけ）
pub const RESIDENCY_TYPES: [&str; 4] = ["resident", "neighbor", "commuter", "related"];

/// 参加者がトピックで投票した承認済みの意見の数と、承認済みの意見の総数。
/// API では `analysis_unlocked` も返す（画面が分析を開く条件を写さずに済むように）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub voted: i64,
    pub total: i64,
}

impl Progress {
    /// 分析（意見マップ・グループ・合意点）を見られるか。
    /// すべての承認済みの意見に投票するまでは見せない（全体の傾向に投票が左右されないように。
    /// 意見が増えれば、また投票が必要になる）
    pub fn analysis_unlocked(&self) -> bool {
        self.voted >= self.total
    }

    pub fn participation(&self) -> Participation {
        if self.voted == 0 {
            Participation::None
        } else if self.voted >= self.total {
            Participation::Completed
        } else {
            Participation::InProgress
        }
    }
}

impl Serialize for Progress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Progress", 3)?;
        s.serialize_field("voted", &self.voted)?;
        s.serialize_field("total", &self.total)?;
        s.serialize_field("analysis_unlocked", &self.analysis_unlocked())?;
        s.end()
    }
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

/// 1つのトピックの進み具合。参加者が分からなければ、投票は0件
pub async fn progress(
    pool: &SqlitePool,
    topic_id: &str,
    user_id: Option<&str>,
) -> Result<Progress, sqlx::Error> {
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
    Ok(Progress { voted, total })
}

/// 参加者がトピックごとに投票した、承認済みの意見の数（自治体の公開済みのトピック）
pub async fn voted_counts(
    pool: &SqlitePool,
    municipality_id: &str,
    user_id: Option<&str>,
) -> Result<HashMap<String, i64>, sqlx::Error> {
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

/// 自治体の公開済みのトピックごとの進み具合（トピックの数だけ問い合わせないように、まとめて数える）
pub async fn progress_by_topic(
    pool: &SqlitePool,
    municipality_id: &str,
    user_id: Option<&str>,
) -> Result<HashMap<String, Progress>, sqlx::Error> {
    let totals: Vec<(String, i64)> = sqlx::query_as(
        "SELECT t.id,
            (SELECT count(*) FROM statements s WHERE s.topic_id = t.id AND s.status = 'approved')
         FROM topics t WHERE t.municipality_id = $1 AND t.status = 'published'",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?;
    let voted = voted_counts(pool, municipality_id, user_id).await?;
    Ok(totals
        .into_iter()
        .map(|(topic_id, total)| {
            let voted = voted.get(&topic_id).copied().unwrap_or(0);
            (topic_id, Progress { voted, total })
        })
        .collect())
}

/// トピックに参加できるか: 指定の自治体に属し、公開済みであること（別の自治体のトピックや下書きは不可）。
/// 投票・投稿・次の意見・分析は、すべてここを通る（API では、できなければ 404）
pub async fn topic_open(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
) -> Result<bool, sqlx::Error> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT id FROM topics WHERE id = $1 AND municipality_id = $2 AND status = 'published'",
    )
    .bind(topic_id)
    .bind(municipality_id)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

/// この自治体との関わり方を登録しているか（投票の前に要る）
pub async fn has_relation(
    pool: &SqlitePool,
    user_id: &str,
    municipality_id: &str,
) -> Result<bool, sqlx::Error> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT user_id FROM user_municipality_relations WHERE user_id = $1 AND municipality_id = $2",
    )
    .bind(user_id)
    .bind(municipality_id)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}
