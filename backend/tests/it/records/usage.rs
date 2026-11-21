// SPDX-License-Identifier: AGPL-3.0-only

//! AI の利用のまとめ（`list usage`）

use backend::records::usage;
use chrono::{Duration, Utc};

use crate::common::test_pool;

async fn insert(
    pool: &sqlx::SqlitePool,
    id: &str,
    purpose: &str,
    outcome: &str,
    cost: i64,
    at: &str,
) {
    sqlx::query(
        "INSERT INTO llm_usage (id, purpose, model, input_tokens, output_tokens,
             cache_creation_input_tokens, cache_read_input_tokens, cost_micro_usd, outcome, created_at)
         VALUES ($1, $2, 'm', 100, 10, 5, 1, $3, $4, $5)",
    )
    .bind(id)
    .bind(purpose)
    .bind(cost)
    .bind(outcome)
    .bind(at)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn daily_groups_by_day_and_purpose_within_range() {
    let pool = test_pool().await;
    let today = Utc::now().date_naive();
    let at =
        |days_ago: i64, time: &str| format!("{}T{time}.000Z", today - Duration::days(days_ago));

    insert(&pool, "a", "moderation", "ok", 1_000, &at(0, "01:00:00")).await;
    insert(
        &pool,
        "b",
        "moderation",
        "invalid",
        2_000,
        &at(0, "23:00:00"),
    )
    .await;
    insert(&pool, "c", "tagging", "ok", 500, &at(0, "02:00:00")).await;
    insert(&pool, "d", "moderation", "ok", 3_000, &at(2, "12:00:00")).await;
    // 範囲の外（3日分なら、3日前は入らない）
    insert(&pool, "e", "moderation", "ok", 9_000, &at(3, "12:00:00")).await;

    let rows = usage::daily(&pool, 3).await.unwrap();
    let day0 = today.to_string();
    let day2 = (today - Duration::days(2)).to_string();
    let summary: Vec<_> = rows
        .iter()
        .map(|r| {
            (
                r.day.as_str(),
                r.purpose.as_str(),
                r.calls,
                r.failures,
                r.cost_micro_usd,
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            (day0.as_str(), "moderation", 2, 1, 3_000),
            (day0.as_str(), "tagging", 1, 0, 500),
            (day2.as_str(), "moderation", 1, 0, 3_000),
        ]
    );
    // 入力のトークンは、キャッシュの書き込み・読み込みも足す
    assert_eq!(rows[1].input_tokens, 106);
    assert_eq!(rows[1].output_tokens, 10);
}
