// SPDX-License-Identifier: AGPL-3.0-only

//! 下書きのトピックは、運営者が公開するまで API のどこからも見えない。

mod common;

use axum::http::StatusCode;
use backend::analysis::store::run_all;
use backend::drafts;
use common::{request, seed_polarized, test_pool, test_state};
use serde_json::json;

const T1: &str = "/api/municipalities/otawara/topics/t1";

#[tokio::test]
async fn draft_is_hidden_until_published() {
    let pool = test_pool().await;
    // 投票と承認済みの意見がある t1 を、下書きにする
    seed_polarized(&pool).await;
    sqlx::query("UPDATE topics SET status = 'draft' WHERE id = 't1'")
        .execute(&pool)
        .await
        .unwrap();
    // 分析のワーカーは下書きを計算しない（計算するのは那須塩原市の t2 だけ）
    assert_eq!(run_all(&pool, true).await.unwrap(), 1);
    let state = test_state(pool.clone());

    let list = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics",
        None,
        None,
    )
    .await;
    assert_eq!(list.json["topics"].as_array().unwrap().len(), 0);
    for path in [
        T1.to_string(),
        format!("{T1}/next-statement"),
        format!("{T1}/analysis"),
    ] {
        let res = request(&state, "GET", &path, None, None).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{path}");
    }
    let vote = request(
        &state,
        "POST",
        &format!("{T1}/statements/a0/votes"),
        None,
        Some(json!({"value": 1})),
    )
    .await;
    assert_eq!(vote.status, StatusCode::NOT_FOUND);
    let post = request(
        &state,
        "POST",
        &format!("{T1}/statements"),
        None,
        Some(json!({"body": "意見"})),
    )
    .await;
    assert_eq!(post.status, StatusCode::NOT_FOUND);

    assert_eq!(drafts::list(&pool).await.unwrap().len(), 1);
    assert!(drafts::set_status(&pool, "t1", "published").await.unwrap());
    assert!(drafts::list(&pool).await.unwrap().is_empty());

    let detail = request(&state, "GET", T1, None, None).await;
    assert_eq!(detail.status, StatusCode::OK);
    assert_eq!(run_all(&pool, true).await.unwrap(), 2);

    // 保管すると、また見えなくなる
    assert!(drafts::set_status(&pool, "t1", "archived").await.unwrap());
    let detail = request(&state, "GET", T1, None, None).await;
    assert_eq!(detail.status, StatusCode::NOT_FOUND);
}
