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

/// 1人が1つのトピックに投稿できる意見の数（ADR 0005。市民提案のトピックの提案者は 6d で別に決める）
pub const STATEMENTS_PER_TOPIC: i64 = 1;

/// 参加者がトピックで投票した承認済みの意見の数と、承認済みの意見の総数（それぞれ、うち最初の意見の数も）。
/// API では `analysis_unlocked` と `analysis_remaining` も返す（画面が分析を開く条件を写さずに済むように）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub voted: i64,
    pub total: i64,
    /// 投票した最初の意見（議会での主張など。`statements.is_seed`）の数
    pub seeds_voted: i64,
    pub seeds_total: i64,
}

impl Progress {
    /// 分析を見られるようになるまでの残りの件数: 未投票の最初の意見の数（ADR 0005）。
    /// 市民の意見は数えない（増えるたびに分析が閉じないように。公開中に足す意見は最初の意見にならない）
    pub fn analysis_remaining(&self) -> i64 {
        (self.seeds_total - self.seeds_voted).max(0)
    }

    /// 分析（意見マップ・グループ・合意点）を見られるか。
    /// 全体の傾向を見る前に、最初の意見（議会での主張など）に自分の考えで投票してもらうため
    pub fn analysis_unlocked(&self) -> bool {
        self.analysis_remaining() == 0
    }

    pub fn participation(&self) -> Participation {
        Participation::of(self.voted, self.total)
    }
}

impl Serialize for Progress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Progress", 4)?;
        s.serialize_field("voted", &self.voted)?;
        s.serialize_field("total", &self.total)?;
        s.serialize_field("analysis_unlocked", &self.analysis_unlocked())?;
        s.serialize_field("analysis_remaining", &self.analysis_remaining())?;
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

impl Participation {
    pub fn of(voted: i64, total: i64) -> Self {
        if voted == 0 {
            Self::None
        } else if voted >= total {
            Self::Completed
        } else {
            Self::InProgress
        }
    }
}

/// 1つのトピックの進み具合。参加者が分からなければ、投票は0件
pub async fn progress(
    pool: &SqlitePool,
    topic_id: &str,
    user_id: Option<&str>,
) -> Result<Progress, sqlx::Error> {
    let (total, seeds_total): (i64, i64) = sqlx::query_as(
        "SELECT count(*), COALESCE(SUM(CASE WHEN is_seed THEN 1 ELSE 0 END), 0)
         FROM statements WHERE topic_id = $1 AND status = 'approved'",
    )
    .bind(topic_id)
    .fetch_one(pool)
    .await?;
    let (voted, seeds_voted): (i64, i64) = match user_id {
        Some(uid) => {
            sqlx::query_as(
                "SELECT count(*), COALESCE(SUM(CASE WHEN s.is_seed THEN 1 ELSE 0 END), 0)
                 FROM votes v
                 JOIN statements s ON s.id = v.statement_id
                 WHERE s.topic_id = $1 AND s.status = 'approved' AND v.user_id = $2",
            )
            .bind(topic_id)
            .bind(uid)
            .fetch_one(pool)
            .await?
        }
        None => (0, 0),
    };
    Ok(Progress {
        voted,
        total,
        seeds_voted,
        seeds_total,
    })
}

/// 投稿できるか（ADR 0005・0006）。API では `{"status": "allowed" | ..., "remaining"?}` で返す
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PostPermission {
    Allowed,
    /// パスキーの登録が要る
    RegistrationRequired,
    /// 分析を見られるようになるまで（最初の意見すべてに）投票する（残りの件数）。ほかの人の意見を読んでから書いてもらうため
    VotesRequired {
        remaining: i64,
    },
    /// このトピックに投稿できる数（`STATEMENTS_PER_TOPIC`）に達した
    LimitReached,
}

impl PostPermission {
    pub fn allowed(self) -> bool {
        self == Self::Allowed
    }
}

/// その参加者がトピックに意見を投稿できるか。
/// 先に投票の条件を見る（未登録の人にも、最初の意見を終えるまではパスキーの案内を出さない）。
/// 数えるのは承認待ち・確認待ち・承認済みの自分の意見。非公開（rejected）は数えない（書き直して出し直せる）
pub async fn post_permission(
    pool: &SqlitePool,
    topic_id: &str,
    user_id: Option<&str>,
) -> Result<PostPermission, sqlx::Error> {
    let progress = progress(pool, topic_id, user_id).await?;
    if !progress.analysis_unlocked() {
        return Ok(PostPermission::VotesRequired {
            remaining: progress.analysis_remaining(),
        });
    }
    let Some(user_id) = user_id else {
        return Ok(PostPermission::RegistrationRequired);
    };
    let registered: Option<bool> =
        sqlx::query_scalar("SELECT registered_at IS NOT NULL FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    if registered != Some(true) {
        return Ok(PostPermission::RegistrationRequired);
    }
    let posted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements
         WHERE topic_id = $1 AND user_id = $2 AND status IN ('pending', 'flagged', 'approved')",
    )
    .bind(topic_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(if posted >= STATEMENTS_PER_TOPIC {
        PostPermission::LimitReached
    } else {
        PostPermission::Allowed
    })
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
    let totals: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT t.id,
            (SELECT count(*) FROM statements s WHERE s.topic_id = t.id AND s.status = 'approved'),
            (SELECT count(*) FROM statements s
             WHERE s.topic_id = t.id AND s.status = 'approved' AND s.is_seed)
         FROM topics t WHERE t.municipality_id = $1 AND t.status = 'published'",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?;
    let voted = voted_counts(pool, municipality_id, user_id).await?;
    let seeds_voted: HashMap<String, i64> = match user_id {
        Some(user_id) => sqlx::query_as(
            "SELECT s.topic_id, count(*) FROM votes v
             JOIN statements s ON s.id = v.statement_id
             JOIN topics t ON t.id = s.topic_id
             WHERE v.user_id = $1 AND s.status = 'approved' AND s.is_seed
               AND t.municipality_id = $2
             GROUP BY s.topic_id",
        )
        .bind(user_id)
        .bind(municipality_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect(),
        None => HashMap::new(),
    };
    Ok(totals
        .into_iter()
        .map(|(topic_id, total, seeds_total)| {
            let progress = Progress {
                voted: voted.get(&topic_id).copied().unwrap_or(0),
                total,
                seeds_voted: seeds_voted.get(&topic_id).copied().unwrap_or(0),
                seeds_total,
            };
            (topic_id, progress)
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
