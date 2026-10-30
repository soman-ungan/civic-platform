// SPDX-License-Identifier: AGPL-3.0-only

//! 市民からのトピックの提案（フェーズ6d。ADR 0005）: 下書きとして保存し、外には出さない。
//! 自治体ごとに1人週1件、提案した本人は意見3件まで、本人の票は自動で入れない（Polis の seed と同じ）。提案した人は API に出さない

use crate::common::{
    activate_second_municipality, assert_no_key, cookie_for, request, seed_fixture, test_pool,
    test_state,
};
use axum::http::StatusCode;
use serde_json::json;
use sqlx::SqlitePool;

const OTAWARA: &str = "/api/municipalities/otawara/proposals";
const NASU: &str = "/api/municipalities/nasushiobara/proposals";

/// u1 を運営者、u2 を提案する人（パスキーを登録し、両方の自治体との関わり方を登録）にする
async fn setup(pool: &SqlitePool) {
    seed_fixture(pool).await;
    activate_second_municipality(pool).await;
    sqlx::raw_sql(
        "UPDATE users SET registered_at = '2026-10-26T00:00:00Z';
         UPDATE users SET is_operator = TRUE WHERE id = 'u1';
         INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at) VALUES
             ('u2', '092100', 'resident', '2026-10-26T00:00:00Z'),
             ('u2', '092134', 'commuter', '2026-10-26T00:00:00Z');
         INSERT INTO public_officials (id, municipality_id, name, role, term_start) VALUES
             ('po1', '092100', '大田 太郎', 'councilor', '2023-01-01');",
    )
    .execute(pool)
    .await
    .unwrap();
}

fn proposal(statements: &[&str]) -> serde_json::Value {
    json!({
        "title": "駅前の駐輪場",
        "description": "駅前の駐輪場が足りない。増やすか、ほかの使い方を考えるか。",
        "statements": statements,
    })
}

async fn propose(state: &backend::interface::api::AppState, path: &str, cookie: &str) -> String {
    let res = request(
        state,
        "POST",
        path,
        Some(cookie),
        Some(proposal(&[
            "駐輪場を増やすべきだ",
            "駅前は歩く人のための広場にすべきだ",
        ])),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
    assert_eq!(res.json["state"], "reviewing");
    res.json["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn proposal_is_saved_as_a_hidden_draft() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let u2 = cookie_for(&state, "u2");
    let id = propose(&state, OTAWARA, &u2).await;

    let (status, source_type, proposed_by): (String, String, String) =
        sqlx::query_as("SELECT status, source_type, proposed_by FROM topics WHERE id = $1")
            .bind(&id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(
        (status.as_str(), source_type.as_str(), proposed_by.as_str()),
        ("draft", "citizen", "u2")
    );
    let seeds: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements
         WHERE topic_id = $1 AND user_id = 'u2' AND status = 'approved' AND is_seed",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(seeds, 2);
    // 提出のときは票を入れない（運営者が意見を直せるように）
    let votes: i64 = sqlx::query_scalar("SELECT count(*) FROM votes")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(votes, 0);

    // 下書きは外に出ない
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics",
        None,
        None,
    )
    .await;
    assert!(!res.json.to_string().contains(&id));
    let res = request(
        &state,
        "GET",
        &format!("/api/municipalities/otawara/topics/{id}"),
        None,
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    // 運営者には市民の提案として見える。提案した人は返さない
    let op = cookie_for(&state, "u1");
    let res = request(
        &state,
        "GET",
        &format!("/api/operator/municipalities/otawara/topics/{id}"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["source_type"], "citizen");
    assert_no_key(&res.json, "proposed_by");
    assert_no_key(&res.json, "user_id");
    assert!(!res.json.to_string().contains("u2"));
}

#[tokio::test]
async fn only_the_proposer_sees_their_proposals() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let u2 = cookie_for(&state, "u2");
    let id = propose(&state, OTAWARA, &u2).await;

    let res = request(&state, "GET", OTAWARA, Some(&u2), None).await;
    let items = res.json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id.as_str());
    assert_eq!(items[0]["state"], "reviewing");
    assert_no_key(&res.json, "proposed_by");

    // ほかの人・Cookie の無い人・別の自治体には出ない
    for cookie in [Some(cookie_for(&state, "u1")), None] {
        let res = request(&state, "GET", OTAWARA, cookie.as_deref(), None).await;
        assert_eq!(res.json["items"].as_array().unwrap().len(), 0);
    }
    let res = request(&state, "GET", NASU, Some(&u2), None).await;
    assert_eq!(res.json["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn one_proposal_per_week_per_municipality() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let u2 = cookie_for(&state, "u2");
    propose(&state, OTAWARA, &u2).await;

    let res = request(&state, "GET", OTAWARA, Some(&u2), None).await;
    assert_eq!(res.json["permission"]["status"], "weekly_limit");
    assert!(res.json["permission"]["next_at"].is_string());
    let res = request(
        &state,
        "POST",
        OTAWARA,
        Some(&u2),
        Some(proposal(&["二つ目"])),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "proposal_limit");

    // 別の自治体には出せる
    propose(&state, NASU, &u2).await;

    // 見送りになっても数える。7日を過ぎれば出せる
    sqlx::query("UPDATE topics SET status = 'archived' WHERE proposed_by = 'u2'")
        .execute(&state.pool)
        .await
        .unwrap();
    let res = request(
        &state,
        "POST",
        OTAWARA,
        Some(&u2),
        Some(proposal(&["出し直し"])),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    sqlx::query(
        "UPDATE topics SET created_at = '2026-01-01T00:00:00.000Z' WHERE proposed_by = 'u2'",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    let res = request(&state, "GET", OTAWARA, Some(&u2), None).await;
    assert_eq!(res.json["permission"]["status"], "allowed");
    assert_eq!(res.json["items"][0]["state"], "declined");
}

#[tokio::test]
async fn proposing_needs_a_passkey_and_a_relation() {
    let pool = test_pool().await;
    setup(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO users (id, created_at) VALUES ('u3', '2026-10-26T00:00:00Z');
         INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at)
             VALUES ('u3', '092100', 'resident', '2026-10-26T00:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = test_state(pool);

    let res = request(&state, "GET", OTAWARA, None, None).await;
    assert_eq!(res.json["permission"]["status"], "registration_required");
    let u3 = cookie_for(&state, "u3");
    let res = request(
        &state,
        "POST",
        OTAWARA,
        Some(&u3),
        Some(proposal(&["意見"])),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.json["error"]["code"], "registration_required");

    // u1 は登録済みだが、関わり方を登録していない
    let u1 = cookie_for(&state, "u1");
    let res = request(&state, "GET", OTAWARA, Some(&u1), None).await;
    assert_eq!(res.json["permission"]["status"], "relation_required");
    let res = request(
        &state,
        "POST",
        OTAWARA,
        Some(&u1),
        Some(proposal(&["意見"])),
    )
    .await;
    assert_eq!(res.json["error"]["code"], "relation_required");

    let topics: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM topics WHERE source_type = 'citizen' AND proposed_by IS NOT NULL",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(topics, 0);
}

#[tokio::test]
async fn proposal_input_is_checked() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let u2 = cookie_for(&state, "u2");
    let long = "あ".repeat(141);
    let cases = [
        json!({"title": "", "description": "説明", "statements": ["意見"]}),
        json!({"title": "タイトル", "description": long, "statements": ["意見"]}),
        json!({"title": "タイトル", "description": "説明", "statements": []}),
        json!({"title": "タイトル", "description": "説明", "statements": (1..=11).map(|i| format!("意見{i}")).collect::<Vec<_>>()}),
        json!({"title": "タイトル", "description": "説明", "statements": [long]}),
    ];
    for body in cases {
        let res = request(&state, "POST", OTAWARA, Some(&u2), Some(body.clone())).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(res.json["error"]["code"], "invalid_proposal");
    }

    // 公人の氏名（空白の有無を問わない）。別の自治体の公人の名簿では見ない
    let named = json!({"title": "タイトル", "description": "説明", "statements": ["大田太郎議員の提案に賛成"]});
    let res = request(&state, "POST", OTAWARA, Some(&u2), Some(named.clone())).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["code"], "official_name");
    let res = request(&state, "POST", NASU, Some(&u2), Some(named)).await;
    assert_eq!(res.status, StatusCode::CREATED);

    let saved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM topics WHERE municipality_id = '092100' AND proposed_by IS NOT NULL",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(saved, 0);
}

#[tokio::test]
async fn proposer_votes_on_their_own_seeds_and_posts_up_to_three() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let u2 = cookie_for(&state, "u2");
    let id = propose(&state, OTAWARA, &u2).await;

    // 運営者が下書きの意見を直せる（まだ票が無い）
    let op = cookie_for(&state, "u1");
    let sid: String =
        sqlx::query_scalar("SELECT id FROM statements WHERE topic_id = $1 ORDER BY body LIMIT 1")
            .bind(&id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let res = request(
        &state,
        "PATCH",
        &format!("/api/operator/municipalities/otawara/statements/{sid}"),
        Some(&op),
        Some(json!({"body": "駅前の駐輪場を増やすべきだ"})),
    )
    .await;
    assert!(res.status.is_success(), "{}", res.json);

    let res = request(
        &state,
        "POST",
        &format!("/api/operator/municipalities/otawara/topics/{id}/publish"),
        Some(&op),
        None,
    )
    .await;
    assert!(res.status.is_success(), "{}", res.json);
    // 公開しても、提案した人の票は入らない（Polis の seed と同じ。反対の立場の意見も書けるように）
    let votes: i64 = sqlx::query_scalar("SELECT count(*) FROM votes")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(votes, 0);

    let res = request(&state, "GET", OTAWARA, Some(&u2), None).await;
    assert_eq!(res.json["items"][0]["state"], "published");

    // 提案した本人も、ほかの人と同じく最初の意見に投票してから投稿する（反対・パスも選べる）
    let path = format!("/api/municipalities/otawara/topics/{id}/next-statement");
    let res = request(&state, "GET", &path, Some(&u2), None).await;
    assert_eq!(res.json["posting"]["status"], "votes_required");
    for value in [1, -1] {
        let res = request(&state, "GET", &path, Some(&u2), None).await;
        let sid = res.json["statement"]["id"].as_str().unwrap().to_owned();
        let res = request(
            &state,
            "POST",
            &format!("/api/municipalities/otawara/topics/{id}/statements/{sid}/votes"),
            Some(&u2),
            Some(json!({"value": value})),
        )
        .await;
        assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    }

    // 提案した本人は、最初の意見とは別に、公開のあと3件まで
    let post = format!("/api/municipalities/otawara/topics/{id}/statements");
    for body in [
        "駐輪場は有料にすべきだ",
        "駐輪場の屋根を直すべきだ",
        "自転車の貸し出しを始めるべきだ",
    ] {
        let res = request(&state, "GET", &path, Some(&u2), None).await;
        assert_eq!(res.json["posting"]["status"], "allowed");
        let res = request(
            &state,
            "POST",
            &post,
            Some(&u2),
            Some(json!({"body": body})),
        )
        .await;
        assert_eq!(res.json["status"], "approved", "{}", res.json);
    }
    let res = request(&state, "GET", &path, Some(&u2), None).await;
    assert_eq!(res.json["posting"]["status"], "limit_reached");

    // 公開のあとに取り下げたら withdrawn
    request(
        &state,
        "POST",
        &format!("/api/operator/municipalities/otawara/topics/{id}/archive"),
        Some(&op),
        None,
    )
    .await;
    let res = request(&state, "GET", OTAWARA, Some(&u2), None).await;
    assert_eq!(res.json["items"][0]["state"], "withdrawn");
}
