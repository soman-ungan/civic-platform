// SPDX-License-Identifier: AGPL-3.0-only

//! 提言の API（ADR 0005）: 公開済みだけを返す、別の自治体のものは 404、下書きは運営者の API にだけ出る。
//! 運営者は作る → 見出しを書く → 公開 → 第2回のトピックの下書き、の順に操作できる

use axum::http::StatusCode;
use backend::process::analysis::store::run_topic;
use serde_json::json;

use crate::common::{
    activate_second_municipality, assert_no_key, cookie_for, request, seed_polarized, test_pool,
    test_state,
};

const OP: &str = "/api/operator/municipalities/otawara";

async fn setup() -> backend::interface::api::AppState {
    let pool = test_pool().await;
    seed_polarized(&pool).await;
    activate_second_municipality(&pool).await;
    run_topic(&pool, "t1", false).await.unwrap();
    sqlx::raw_sql(
        "UPDATE topics SET closes_at = '2026-01-01T00:00:00.000Z' WHERE id = 't1';
         UPDATE users SET is_operator = TRUE, registered_at = '2026-10-26T00:00:00Z' WHERE id = 'u1';",
    )
    .execute(&pool)
    .await
    .unwrap();
    test_state(pool)
}

#[tokio::test]
async fn operators_make_publish_and_start_a_second_round() {
    let state = setup().await;
    let op = cookie_for(&state, "u1");
    let citizen = cookie_for(&state, "u2");

    // 運営者でなければ 404
    for cookie in [None, Some(citizen.as_str())] {
        let res = request(
            &state,
            "GET",
            &format!("{OP}/recommendation-candidates"),
            cookie,
            None,
        )
        .await;
        assert_eq!(res.status, StatusCode::NOT_FOUND);
    }

    let res = request(
        &state,
        "GET",
        &format!("{OP}/recommendation-candidates"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["topics"][0]["id"], "t1");

    let res = request(
        &state,
        "POST",
        &format!("{OP}/recommendations"),
        Some(&op),
        Some(json!({"topic_ids": ["t1"]})),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let id = res.json["id"].as_str().unwrap().to_owned();
    // API キーが無いので、見出しは運営者が書く
    assert_eq!(res.json["title"], "");
    assert!(res.json["text_error"].is_null());

    // 下書きは公開の API に出ない
    let public = format!("/api/municipalities/otawara/recommendations/{id}");
    assert_eq!(
        request(&state, "GET", &public, None, None).await.status,
        StatusCode::NOT_FOUND
    );
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/recommendations",
        None,
        None,
    )
    .await;
    assert_eq!(res.json.as_array().unwrap().len(), 0);

    let res = request(
        &state,
        "POST",
        &format!("{OP}/recommendations/{id}/publish"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = request(
        &state,
        "PATCH",
        &format!("{OP}/recommendations/{id}"),
        Some(&op),
        Some(json!({"title": "テストの提言", "preface": "前書き"})),
    )
    .await;
    assert_eq!(res.json["title"], "テストの提言");
    let res = request(
        &state,
        "POST",
        &format!("{OP}/recommendations/{id}/publish"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["status"], "published");

    let res = request(&state, "GET", &public, None, None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["items"].as_array().unwrap().len(), 7);
    assert_no_key(&res.json, "user_id");
    // 別の自治体の URL では見えない
    let other = format!("/api/municipalities/nasushiobara/recommendations/{id}");
    assert_eq!(
        request(&state, "GET", &other, None, None).await.status,
        StatusCode::NOT_FOUND
    );

    // 公開した提言は直せない・消せない
    let res = request(
        &state,
        "PATCH",
        &format!("{OP}/recommendations/{id}"),
        Some(&op),
        Some(json!({"title": "直す"})),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    let res = request(
        &state,
        "DELETE",
        &format!("{OP}/recommendations/{id}"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);

    // トピックの画面から提言へのリンク
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics/t1",
        None,
        None,
    )
    .await;
    assert_eq!(res.json["recommendations"][0]["id"], id);

    // 第2回のトピックの下書き。公開したら、元の提言へのリンクが出る
    let res = request(
        &state,
        "POST",
        &format!("{OP}/recommendations/{id}/second-round"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let topic = res.json["topic_id"].as_str().unwrap().to_owned();
    let res = request(
        &state,
        "POST",
        &format!("{OP}/topics/{topic}/publish"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let res = request(
        &state,
        "GET",
        &format!("/api/municipalities/otawara/topics/{topic}"),
        None,
        None,
    )
    .await;
    assert_eq!(res.json["origin"]["id"], id);
    let res = request(&state, "GET", &public, None, None).await;
    assert_eq!(res.json["second_round"][0]["id"], topic);
}

#[tokio::test]
async fn other_municipality_cannot_be_mixed() {
    let state = setup().await;
    let op = cookie_for(&state, "u1");
    let res = request(
        &state,
        "POST",
        "/api/operator/municipalities/nasushiobara/recommendations",
        Some(&op),
        Some(json!({"topic_ids": ["t1"]})),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = request(
        &state,
        "POST",
        &format!("{OP}/recommendations"),
        Some(&op),
        Some(json!({"topic_ids": ["t1"]})),
    )
    .await;
    let id = res.json["id"].as_str().unwrap();
    let res = request(
        &state,
        "GET",
        &format!("/api/operator/municipalities/nasushiobara/recommendations/{id}"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}
