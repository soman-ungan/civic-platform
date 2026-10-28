// SPDX-License-Identifier: AGPL-3.0-only

//! 参加の決まり（フェーズ6b。ADR 0005）: 分析を見られる条件、次の意見は最初の意見から、
//! 投稿は分析を見られてから・1トピック1件まで。ルールは `process::progress`

mod common;

use axum::http::StatusCode;
use backend::interface::api::AppState;
use backend::process::progress::{ANALYSIS_UNLOCK_VOTES, Progress};
use common::{cookie_for, insert_vote, request, seed_fixture, test_pool, test_state};
use serde_json::{Value, json};
use sqlx::SqlitePool;

const T1: &str = "/api/municipalities/otawara/topics/t1";
const AT: &str = "2026-10-28T00:00:00Z";

/// t1 に最初の意見 `seeds` 件（seed0..）と、市民の意見 `others` 件（c0..）を承認済みで入れる
async fn setup(seeds: usize, others: usize) -> (SqlitePool, AppState) {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    // fixture の s1（u1 の pending の意見）は数に入らない
    for (prefix, n, is_seed) in [("seed", seeds, true), ("c", others, false)] {
        for i in 0..n {
            sqlx::query(
                "INSERT INTO statements (id, topic_id, body, status, is_seed, created_at)
                 VALUES ($1, 't1', $2, 'approved', $3, $4)",
            )
            .bind(format!("{prefix}{i}"))
            .bind(format!("{prefix} の意見 {i}"))
            .bind(is_seed)
            .bind(AT)
            .execute(&pool)
            .await
            .unwrap();
        }
    }
    let state = test_state(pool.clone());
    (pool, state)
}

/// パスキーを登録し、関わり方も登録した参加者（ID と Cookie）
async fn participant(pool: &SqlitePool, state: &AppState, id: &str) -> String {
    sqlx::raw_sql(&format!(
        "INSERT INTO users (id, created_at, registered_at) VALUES ('{id}', '{AT}', '{AT}');
         INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at)
         VALUES ('{id}', '092100', 'resident', '{AT}');"
    ))
    .execute(pool)
    .await
    .unwrap();
    cookie_for(state, id)
}

async fn votes(pool: &SqlitePool, user: &str, ids: &[String]) {
    for sid in ids {
        insert_vote(pool, user, sid, 1, AT).await;
    }
}

fn ids(prefix: &str, range: std::ops::Range<usize>) -> Vec<String> {
    range.map(|i| format!("{prefix}{i}")).collect()
}

async fn next(state: &AppState, cookie: &str) -> Value {
    request(
        state,
        "GET",
        &format!("{T1}/next-statement"),
        Some(cookie),
        None,
    )
    .await
    .json
}

async fn post(state: &AppState, cookie: &str, body: &str) -> common::TestResponse {
    request(
        state,
        "POST",
        &format!("{T1}/statements"),
        Some(cookie),
        Some(json!({ "body": body })),
    )
    .await
}

#[test]
fn unlock_needs_all_seeds_and_the_fixed_number() {
    let p = |voted, total, seeds_voted, seeds_total| Progress {
        voted,
        total,
        seeds_voted,
        seeds_total,
    };
    assert_eq!(ANALYSIS_UNLOCK_VOTES, 15);
    // 最初の意見が残っていれば、15件に投票していても開かない
    assert_eq!(p(15, 30, 2, 3).analysis_remaining(), 1);
    // 最初の意見を終えても、15件に届くまで開かない
    assert_eq!(p(10, 30, 3, 3).analysis_remaining(), 5);
    assert!(p(15, 30, 3, 3).analysis_unlocked());
    // 意見が15件より少なければ、全部で開く
    assert_eq!(p(4, 6, 2, 2).analysis_remaining(), 2);
    assert!(p(6, 6, 2, 2).analysis_unlocked());
    // 意見が無いトピックは、すぐに開く（最初の投稿もできる）
    assert!(p(0, 0, 0, 0).analysis_unlocked());
}

#[tokio::test]
async fn analysis_and_progress_use_the_rule() {
    let (pool, state) = setup(3, 20).await;
    let cookie = participant(&pool, &state, "p1").await;

    votes(&pool, "p1", &ids("c", 0..15)).await;
    let json = next(&state, &cookie).await;
    assert_eq!(json["progress"]["analysis_unlocked"], false);
    assert_eq!(json["progress"]["analysis_remaining"], 3);
    let analysis = request(
        &state,
        "GET",
        &format!("{T1}/analysis"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(analysis.json["status"], "locked");
    assert_eq!(analysis.json["remaining"], 3);

    votes(&pool, "p1", &ids("seed", 0..3)).await;
    let json = next(&state, &cookie).await;
    assert_eq!(json["progress"]["analysis_unlocked"], true);
    assert_eq!(json["progress"]["analysis_remaining"], 0);
    let analysis = request(
        &state,
        "GET",
        &format!("{T1}/analysis"),
        Some(&cookie),
        None,
    )
    .await;
    assert_ne!(analysis.json["status"], "locked");
}

#[tokio::test]
async fn seeds_come_first() {
    let (pool, state) = setup(3, 20).await;
    let cookie = participant(&pool, &state, "p1").await;
    for _ in 0..3 {
        let json = next(&state, &cookie).await;
        let id = json["statement"]["id"].as_str().unwrap().to_owned();
        assert!(id.starts_with("seed"), "最初の意見が先に出る: {id}");
        insert_vote(&pool, "p1", &id, 0, AT).await;
    }
    let json = next(&state, &cookie).await;
    assert!(json["statement"]["id"].as_str().unwrap().starts_with('c'));
    // API では最初の意見かどうかを返さない
    common::assert_no_key(&json, "is_seed");
}

#[tokio::test]
async fn posting_waits_for_the_analysis_and_is_one_per_topic() {
    let (pool, state) = setup(2, 20).await;
    let cookie = participant(&pool, &state, "p1").await;

    votes(&pool, "p1", &ids("seed", 0..2)).await;
    votes(&pool, "p1", &ids("c", 0..10)).await;
    assert_eq!(
        next(&state, &cookie).await["posting"],
        json!({"status": "votes_required", "remaining": 3})
    );
    let res = post(&state, &cookie, "公園を増やしてほしい").await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.json["error"]["code"], "votes_required");

    votes(&pool, "p1", &ids("c", 10..13)).await;
    assert_eq!(
        next(&state, &cookie).await["posting"],
        json!({"status": "allowed"})
    );
    assert_eq!(
        post(&state, &cookie, "公園を増やしてほしい").await.status,
        StatusCode::CREATED
    );

    // 2件目はできない
    assert_eq!(
        next(&state, &cookie).await["posting"],
        json!({"status": "limit_reached"})
    );
    let res = post(&state, &cookie, "図書館も増やしてほしい").await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "statement_limit");

    // 非公開になった意見は数えない（書き直して出し直せる）
    sqlx::query("UPDATE statements SET status = 'rejected' WHERE user_id = 'p1'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        post(&state, &cookie, "図書館も増やしてほしい").await.status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn pending_and_flagged_count_but_other_topics_do_not() {
    let (pool, state) = setup(0, 3).await;
    let cookie = participant(&pool, &state, "p1").await;
    votes(&pool, "p1", &ids("c", 0..3)).await;
    for status in ["pending", "flagged"] {
        sqlx::query("DELETE FROM statements WHERE user_id = 'p1'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO statements (id, topic_id, user_id, body, status, created_at)
             VALUES ('mine', 't1', 'p1', '自分の意見', $1, $2)",
        )
        .bind(status)
        .bind(AT)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            next(&state, &cookie).await["posting"]["status"],
            "limit_reached",
            "{status} も数える"
        );
    }
    // 別のトピックの意見は数えない
    sqlx::query("UPDATE statements SET topic_id = 't2' WHERE id = 'mine'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(next(&state, &cookie).await["posting"]["status"], "allowed");
}

#[tokio::test]
async fn anonymous_participants_see_registration_required() {
    let (pool, state) = setup(0, 1).await;
    sqlx::query("INSERT INTO users (id, created_at) VALUES ('anon', $1)")
        .bind(AT)
        .execute(&pool)
        .await
        .unwrap();
    let cookie = cookie_for(&state, "anon");
    assert_eq!(
        next(&state, &cookie).await["posting"],
        json!({"status": "registration_required"})
    );
}
