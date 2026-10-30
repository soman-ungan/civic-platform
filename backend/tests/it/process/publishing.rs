// SPDX-License-Identifier: AGPL-3.0-only

//! 下書きの自動公開（docs/plans/2026-10-30-timeline-flow.md）: 時刻が来るたびに、
//! 自治体ごとの公開待ちの古い順に1件を公開する。その日のうちなら逃した時刻も取り戻す。決まりは `process::publishing`

use crate::common::{request, seed_fixture, test_pool, test_state};
use backend::process::publishing::{add_slot, run_due};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// 大田原市に下書き q1・q2（q1 を先に公開待ち）と待ちに入れていない q3、那須塩原市に公開待ちの n1
async fn setup(pool: &SqlitePool) {
    seed_fixture(pool).await;
    sqlx::raw_sql(
        "UPDATE municipalities SET is_active = TRUE;
         INSERT INTO topics (id, municipality_id, title, source_type, status, queued_at, created_at) VALUES
             ('q2', '092100', '二番目', 'assembly', 'draft', '2026-10-29T02:00:00.000Z', '2026-10-01T00:00:00.000Z'),
             ('q1', '092100', '一番目', 'assembly', 'draft', '2026-10-29T01:00:00.000Z', '2026-10-02T00:00:00.000Z'),
             ('q3', '092100', '待ちでない', 'assembly', 'draft', NULL, '2026-09-01T00:00:00.000Z'),
             ('n1', '092134', '那須塩原', 'assembly', 'draft', '2026-10-29T00:00:00.000Z', '2026-10-01T00:00:00.000Z');",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn status(pool: &SqlitePool, id: &str) -> (String, Option<String>, Option<String>) {
    sqlx::query_as("SELECT status, queued_at, published_at FROM topics WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn publishes_one_per_slot_in_queue_order() {
    let pool = test_pool().await;
    setup(&pool).await;
    // 日本時間 2026-10-30 06:00 に 07:00 と 12:00 を足す
    let before = utc("2026-10-29T21:00:00Z");
    add_slot(&pool, "092100", "7:00", before).await.unwrap();
    add_slot(&pool, "092100", "12:00", before).await.unwrap();

    // 06:30 はまだ何もしない
    assert!(
        run_due(&pool, utc("2026-10-29T21:30:00Z"))
            .await
            .unwrap()
            .is_empty()
    );
    // 07:01: 先に入れた q1 だけ
    assert_eq!(
        run_due(&pool, utc("2026-10-29T22:01:00Z")).await.unwrap(),
        ["q1"]
    );
    let (s, queued, published) = status(&pool, "q1").await;
    assert_eq!(s, "published");
    assert!(queued.is_none() && published.is_some());
    // 同じ時刻の枠は二度動かない
    assert!(
        run_due(&pool, utc("2026-10-29T22:30:00Z"))
            .await
            .unwrap()
            .is_empty()
    );
    // 12:00 に q2。待ちに入れていない q3 と、別の自治体の n1 は残る
    assert_eq!(
        run_due(&pool, utc("2026-10-30T03:00:00Z")).await.unwrap(),
        ["q2"]
    );
    assert_eq!(status(&pool, "q3").await.0, "draft");
    assert_eq!(status(&pool, "n1").await.0, "draft");
    // 待ちが空なら、次の日の時刻が来ても何もしない
    assert!(
        run_due(&pool, utc("2026-10-30T22:01:00Z"))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn catches_up_within_the_day_but_not_across_days() {
    let pool = test_pool().await;
    setup(&pool).await;
    add_slot(&pool, "092100", "07:00", utc("2026-10-29T21:00:00Z"))
        .await
        .unwrap();
    // 07:00 に止まっていて、日本時間 15:00 に動いた: その日のうちなら公開する
    assert_eq!(
        run_due(&pool, utc("2026-10-30T06:00:00Z")).await.unwrap(),
        ["q1"]
    );
    // 2日止まっていた: 逃した日の分はまとめて出さず、その日の1件だけ
    assert_eq!(
        run_due(&pool, utc("2026-11-02T06:00:00Z")).await.unwrap(),
        ["q2"]
    );
}

#[tokio::test]
async fn a_slot_added_after_its_time_starts_tomorrow() {
    let pool = test_pool().await;
    setup(&pool).await;
    // 日本時間 10:00 に 07:00 を足しても、今日は公開しない
    let now = utc("2026-10-30T01:00:00Z");
    add_slot(&pool, "092100", "07:00", now).await.unwrap();
    assert!(run_due(&pool, now).await.unwrap().is_empty());
    assert_eq!(
        run_due(&pool, utc("2026-10-30T22:00:00Z")).await.unwrap(),
        ["q1"]
    );
}

#[tokio::test]
async fn queued_drafts_stay_hidden_until_published() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics",
        None,
        None,
    )
    .await;
    let ids: Vec<&str> = res.json["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.get("id").unwrap().as_str().unwrap())
        .collect();
    assert!(!ids.contains(&"q1"));
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics/q1",
        None,
        None,
    )
    .await;
    assert_eq!(res.status, axum::http::StatusCode::NOT_FOUND);
}
