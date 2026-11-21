// SPDX-License-Identifier: AGPL-3.0-only

//! AI（Claude API）の利用の記録（`llm_usage`）を日ごと・用途ごとにまとめる（`list usage`）。
//! 本番のコンテナには sqlite3 が無いので、キーを差し替えたあとや費用の確かめに使う

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

/// 1日（UTC）・1つの用途のまとめ
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct DailyUsage {
    /// YYYY-MM-DD（UTC）
    pub day: String,
    /// moderation / tagging / eval / extraction
    pub purpose: String,
    pub calls: i64,
    /// 正常に答えなかった回数（refusal・max_tokens・invalid）
    pub failures: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    /// 推定の費用（100万分の1ドル）
    pub cost_micro_usd: i64,
}

/// 直近 `days` 日（今日を含む。UTC）の利用を、新しい日から用途の順に返す
pub async fn daily(pool: &SqlitePool, days: u32) -> Result<Vec<DailyUsage>, sqlx::Error> {
    let first = Utc::now().date_naive() - Duration::days(i64::from(days.max(1)) - 1);
    sqlx::query_as(
        "SELECT SUBSTR(created_at, 1, 10) AS day, purpose,
                COUNT(*) AS calls,
                SUM(CASE WHEN outcome = 'ok' THEN 0 ELSE 1 END) AS failures,
                SUM(input_tokens + cache_creation_input_tokens + cache_read_input_tokens) AS input_tokens,
                SUM(output_tokens) AS output_tokens,
                SUM(cost_micro_usd) AS cost_micro_usd
         FROM llm_usage
         WHERE created_at >= $1
         GROUP BY SUBSTR(created_at, 1, 10), purpose
         ORDER BY day DESC, purpose",
    )
    .bind(format!("{first}T00:00:00.000Z"))
    .fetch_all(pool)
    .await
}
