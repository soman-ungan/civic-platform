// SPDX-License-Identifier: AGPL-3.0-only

//! トピックの期限（フェーズ6c。ADR 0005）: 締め切ったら投票と投稿を止め、結果は誰でも見られる。
//! ルールは `process::progress`

mod common;

use axum::http::StatusCode;
use backend::process::analysis::store::run_topic;
use backend::process::progress::{default_closes_at, is_closed, normalize_closes_at};
use common::{account_ready_to_post, cookie_for, request, seed_polarized, test_pool, test_state};
use serde_json::json;
use sqlx::SqlitePool;

const T1: &str = "/api/municipalities/otawara/topics/t1";
const PAST: &str = "2026-01-01T00:00:00.000Z";

async fn set_closes_at(pool: &SqlitePool, topic_id: &str, closes_at: Option<&str>) {
    sqlx::query("UPDATE topics SET closes_at = $1 WHERE id = $2")
        .bind(closes_at)
        .bind(topic_id)
        .execute(pool)
        .await
        .unwrap();
}

#[test]
fn deadlines_are_compared_in_one_format() {
    assert!(is_closed(Some(PAST)));
    assert!(!is_closed(Some(&default_closes_at())));
    // 期限なしは締め切らない
    assert!(!is_closed(None));
    // 入力の形をそろえる（タイムゾーンつきなら UTC に）。読めなければ None
    assert_eq!(
        normalize_closes_at("2026-12-01T09:00:00+09:00").as_deref(),
        Some("2026-12-01T00:00:00.000Z")
    );
    assert_eq!(normalize_closes_at("2026-12-01"), None);
}

#[tokio::test]
async fn closed_topics_stop_votes_and_posts_but_show_results_to_everyone() {
    let pool = test_pool().await;
    seed_polarized(&pool).await;
    // 最初の意見があると、投票していない人には分析が閉じている
    sqlx::query("UPDATE statements SET is_seed = TRUE WHERE topic_id = 't1'")
        .execute(&pool)
        .await
        .unwrap();
    run_topic(&pool, "t1", false).await.unwrap();
    let state = test_state(pool.clone());
    let poster = account_ready_to_post(&pool, &state, "t1").await;
    let analysis = request(&state, "GET", &format!("{T1}/analysis"), None, None).await;
    assert_eq!(analysis.json["status"], "locked");

    set_closes_at(&pool, "t1", Some(PAST)).await;

    // 投票できない（関わり方の登録より先に断る）
    let voter = cookie_for(&state, "v00");
    let res = request(
        &state,
        "POST",
        &format!("{T1}/statements/a0/votes"),
        Some(&voter),
        Some(json!({ "value": 1 })),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "topic_closed");

    // 投稿できない
    let res = request(
        &state,
        "POST",
        &format!("{T1}/statements"),
        Some(&poster),
        Some(json!({ "body": "公園を増やしてほしい" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "topic_closed");

    // 次の意見は出さず、締め切ったことを返す
    let next = request(&state, "GET", &format!("{T1}/next-statement"), None, None).await;
    assert!(next.json["statement"].is_null());
    assert_eq!(next.json["progress"]["closed"], true);
    assert_eq!(next.json["progress"]["analysis_unlocked"], true);
    assert_eq!(next.json["posting"], json!({ "status": "closed" }));

    // 結果は、投票していない人も見られる
    let analysis = request(&state, "GET", &format!("{T1}/analysis"), None, None).await;
    assert_eq!(analysis.json["status"], "ready");

    // 詳細と一覧にも出す
    let detail = request(&state, "GET", T1, None, None).await;
    assert_eq!(detail.json["closed"], true);
    assert_eq!(detail.json["closes_at"], PAST);
}

#[tokio::test]
async fn open_topics_come_before_closed_ones() {
    let pool = test_pool().await;
    common::seed_fixture(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO topics (id, municipality_id, title, source_type, created_at) VALUES
            ('t-old', '092100', '古い受付中', 'citizen', '2026-10-01T00:00:00Z'),
            ('t-new-closed', '092100', '新しいが締め切り', 'citizen', '2026-10-27T00:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    set_closes_at(&pool, "t-new-closed", Some(PAST)).await;
    let state = test_state(pool);
    let list = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics",
        None,
        None,
    )
    .await;
    let ids: Vec<&str> = list.json["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["t1", "t-old", "t-new-closed"]);
    assert_eq!(list.json["topics"][0]["closed"], false);
    assert_eq!(list.json["topics"][2]["closed"], true);
}
