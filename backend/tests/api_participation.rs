// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use axum::http::StatusCode;
use backend::interface::api::AppState;
use common::{assert_no_key, request};
use serde_json::{Value, json};
use sqlx::SqlitePool;

const T1: &str = "/api/municipalities/otawara/topics/t1";

/// fixture に加えて、t1 に承認済みの意見 a1〜a3 と未承認の p1、t2 に承認済みの b1 を入れる
async fn setup() -> (SqlitePool, AppState) {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    sqlx::raw_sql(
        "
        INSERT INTO statements (id, topic_id, body, status, created_at) VALUES
            ('a1', 't1', '承認済み1', 'approved', '2026-10-26T00:00:00Z'),
            ('a2', 't1', '承認済み2', 'approved', '2026-10-26T00:00:00Z'),
            ('a3', 't1', '承認済み3', 'approved', '2026-10-26T00:00:00Z'),
            ('p1', 't1', '保留中', 'pending', '2026-10-26T00:00:00Z'),
            ('b1', 't2', '那須塩原の意見', 'approved', '2026-10-26T00:00:00Z');
        ",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = common::test_state(pool.clone());
    (pool, state)
}

/// 居住地属性を登録した参加者の Cookie を返す
async fn registered(state: &AppState) -> String {
    let res = request(
        state,
        "PUT",
        "/api/municipalities/otawara/relation",
        None,
        Some(json!({ "residency_type": "resident" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    res.cookie.expect("新しい参加者には Cookie が発行される")
}

async fn vote(
    state: &AppState,
    cookie: Option<&str>,
    statement_id: &str,
    value: i64,
) -> common::TestResponse {
    request(
        state,
        "POST",
        &format!("{T1}/statements/{statement_id}/votes"),
        cookie,
        Some(json!({ "value": value })),
    )
    .await
}

async fn next(state: &AppState, cookie: Option<&str>) -> Value {
    let res = request(state, "GET", &format!("{T1}/next-statement"), cookie, None).await;
    assert_eq!(res.status, StatusCode::OK);
    res.json
}

// ===== 居住地属性 =====

#[tokio::test]
async fn relation_does_not_collect_district() {
    let (pool, state) = setup().await;
    // 古いクライアントが地区を送ってきても、受け取らず保存もしない（ADR 0002）
    let res = request(
        &state,
        "PUT",
        "/api/municipalities/otawara/relation",
        None,
        Some(json!({ "residency_type": "resident", "district_id": "092100-kurobane" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json, json!({ "residency_type": "resident" }));
    let districts: Vec<Option<String>> =
        sqlx::query_scalar("SELECT district_id FROM user_municipality_relations")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(districts, [None]);
}

#[tokio::test]
async fn relation_rejects_unknown_residency_type() {
    let (_pool, state) = setup().await;
    let res = request(
        &state,
        "PUT",
        "/api/municipalities/otawara/relation",
        None,
        Some(json!({ "residency_type": "external" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["code"], "invalid_residency_type");
}

#[tokio::test]
async fn relation_can_be_updated() {
    let (pool, state) = setup().await;
    let cookie = registered(&state).await;
    let res = request(
        &state,
        "PUT",
        "/api/municipalities/otawara/relation",
        Some(&cookie),
        Some(json!({ "residency_type": "commuter" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let types: Vec<String> =
        sqlx::query_scalar("SELECT residency_type FROM user_municipality_relations")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(types, ["commuter"]);
}

// ===== 投票 =====

#[tokio::test]
async fn vote_requires_relation() {
    let (pool, state) = setup().await;
    let res = vote(&state, None, "a1", 1).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "relation_required");
    assert!(res.cookie.is_none());
    assert_eq!(
        user_count(&pool).await,
        2,
        "fixture の u1, u2 以外にユーザーは作られない"
    );
}

#[tokio::test]
async fn invalid_requests_do_not_create_users() {
    let (pool, state) = setup().await;
    // 141 文字の投稿・不正な居住地属性・存在しないトピックへの投稿
    post(&state, None, &"あ".repeat(141)).await;
    request(
        &state,
        "PUT",
        "/api/municipalities/otawara/relation",
        None,
        Some(json!({ "residency_type": "external" })),
    )
    .await;
    request(
        &state,
        "POST",
        "/api/municipalities/otawara/topics/missing/statements",
        None,
        Some(json!({ "body": "意見" })),
    )
    .await;
    assert_eq!(user_count(&pool).await, 2);
}

async fn user_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn vote_is_recorded_and_overwritten() {
    let (pool, state) = setup().await;
    let cookie = registered(&state).await;

    let res = vote(&state, Some(&cookie), "a1", 1).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json, json!({ "statement_id": "a1", "value": 1 }));

    let res = vote(&state, Some(&cookie), "a1", -1).await;
    assert_eq!(res.status, StatusCode::OK);

    let values: Vec<i64> =
        sqlx::query_scalar("SELECT vote_value FROM votes WHERE statement_id = 'a1'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(values, [-1], "1人1票のまま、最後の判断で上書きされる");
}

#[tokio::test]
async fn vote_value_is_validated() {
    let (_pool, state) = setup().await;
    let cookie = registered(&state).await;
    let res = vote(&state, Some(&cookie), "a1", 2).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["code"], "invalid_vote_value");
}

#[tokio::test]
async fn vote_on_pending_or_other_topic_statement_is_404() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;
    let cookie = registered(&state).await;

    // 未承認の意見
    assert_eq!(
        vote(&state, Some(&cookie), "p1", 1).await.status,
        StatusCode::NOT_FOUND
    );
    // 別の自治体のトピックの意見を、大田原の t1 の URL で指定
    assert_eq!(
        vote(&state, Some(&cookie), "b1", 1).await.status,
        StatusCode::NOT_FOUND
    );
    // 大田原の URL で那須塩原のトピックを指定
    let res = request(
        &state,
        "POST",
        "/api/municipalities/otawara/topics/t2/statements/b1/votes",
        Some(&cookie),
        Some(json!({ "value": 1 })),
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn relation_is_per_municipality() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;
    let cookie = registered(&state).await;

    // 大田原で登録しただけでは、那須塩原では投票できない
    let res = request(
        &state,
        "POST",
        "/api/municipalities/nasushiobara/topics/t2/statements/b1/votes",
        Some(&cookie),
        Some(json!({ "value": 1 })),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn non_json_body_is_rejected_with_api_error() {
    let (_pool, state) = setup().await;
    let res = request(&state, "POST", &format!("{T1}/statements"), None, None).await;
    assert_eq!(res.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(res.json["error"]["code"], "invalid_request");
}

// ===== 次の意見 =====

#[tokio::test]
async fn next_statement_for_new_visitor() {
    let (_pool, state) = setup().await;
    let json = next(&state, None).await;
    assert_eq!(json["needs_relation"], true);
    assert_eq!(
        json["progress"],
        json!({ "voted": 0, "total": 3, "analysis_unlocked": false })
    );
    let id = json["statement"]["id"].as_str().unwrap();
    assert!(
        ["a1", "a2", "a3"].contains(&id),
        "承認済みの t1 の意見だけが出る: {id}"
    );
}

#[tokio::test]
async fn next_statement_skips_voted_and_ends_with_null() {
    let (_pool, state) = setup().await;
    let cookie = registered(&state).await;

    let mut seen = Vec::new();
    for _ in 0..3 {
        let json = next(&state, Some(&cookie)).await;
        assert_eq!(json["needs_relation"], false);
        let id = json["statement"]["id"].as_str().unwrap().to_owned();
        assert!(!seen.contains(&id), "投票済みの意見は出ない");
        assert_eq!(
            vote(&state, Some(&cookie), &id, 0).await.status,
            StatusCode::OK
        );
        seen.push(id);
    }

    let json = next(&state, Some(&cookie)).await;
    assert_eq!(json["statement"], Value::Null);
    assert_eq!(
        json["progress"],
        json!({ "voted": 3, "total": 3, "analysis_unlocked": true })
    );
}

#[tokio::test]
async fn next_statement_of_other_municipality_topic_is_404() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics/t2/next-statement",
        None,
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

// ===== 意見の投稿 =====

#[tokio::test]
async fn posting_requires_a_passkey_but_voting_does_not() {
    let (pool, state) = setup().await;
    // 匿名の参加者は投票できる
    let cookie = registered(&state).await;
    assert_eq!(
        vote(&state, Some(&cookie), "a1", 1).await.status,
        StatusCode::OK
    );
    // 投稿は、パスキーを登録するまでできない（Cookie が無くても、匿名でも）
    for c in [None, Some(cookie.as_str())] {
        let res = post(&state, c, "公園をもっと増やしてほしい").await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        assert_eq!(res.json["error"]["code"], "registration_required");
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM statements WHERE user_id IS NOT NULL AND id NOT IN ('s1')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

async fn post(state: &AppState, cookie: Option<&str>, body: &str) -> common::TestResponse {
    request(
        state,
        "POST",
        &format!("{T1}/statements"),
        cookie,
        Some(json!({ "body": body })),
    )
    .await
}

#[tokio::test]
async fn posted_statement_is_auto_approved_and_appears_in_cards() {
    let (pool, state) = setup().await;
    let cookie = registered(&state).await;
    for id in ["a1", "a2", "a3"] {
        vote(&state, Some(&cookie), id, 1).await;
    }

    let account = common::registered_account(&pool, &state).await;
    let res = post(&state, Some(&account), "  公園をもっと増やしてほしい  ").await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.json["status"], "approved");
    assert!(res.json.get("suggestion").is_none());
    let new_id = res.json["id"].as_str().unwrap().to_owned();

    // 前後の空白は除いて保存される
    let body: String = sqlx::query_scalar("SELECT body FROM statements WHERE id = $1")
        .bind(&new_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(body, "公園をもっと増やしてほしい");

    // 投票を終えていた人にも、新しい意見が次のカードとして出る
    let json = next(&state, Some(&cookie)).await;
    assert_eq!(json["statement"]["id"], new_id);
    assert_eq!(json["statement"]["body"], "公園をもっと増やしてほしい");
}

#[tokio::test]
async fn statement_length_is_counted_in_characters() {
    let (pool, state) = setup().await;
    let account = common::registered_account(&pool, &state).await;
    // 絵文字は UTF-8 で 4 バイトだが 1 文字として数える（DB の length() と一致）
    assert_eq!(
        post(&state, Some(&account), &"😀".repeat(140)).await.status,
        StatusCode::CREATED
    );
    assert_eq!(
        post(&state, Some(&account), &"あ".repeat(140)).await.status,
        StatusCode::CREATED
    );

    let too_long = post(&state, Some(&account), &"😀".repeat(141)).await;
    assert_eq!(too_long.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(too_long.json["error"]["code"], "invalid_body");

    assert_eq!(
        post(&state, Some(&account), "   \n  ").await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn statement_to_other_municipality_topic_is_404() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;
    let res = request(
        &state,
        "POST",
        "/api/municipalities/otawara/topics/t2/statements",
        None,
        Some(json!({ "body": "意見" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

// ===== 匿名性 =====

#[tokio::test]
async fn participation_responses_have_no_user_id() {
    let (_pool, state) = setup().await;
    let cookie = registered(&state).await;

    let responses = [
        next(&state, None).await,
        next(&state, Some(&cookie)).await,
        vote(&state, Some(&cookie), "a1", 1).await.json,
        post(&state, Some(&cookie), "意見です").await.json,
        request(
            &state,
            "PUT",
            "/api/municipalities/otawara/relation",
            Some(&cookie),
            Some(json!({ "residency_type": "related" })),
        )
        .await
        .json,
        request(
            &state,
            "GET",
            "/api/municipalities/otawara/topics",
            Some(&cookie),
            None,
        )
        .await
        .json,
    ];
    for json in &responses {
        assert_no_key(json, "user_id");
    }
}
