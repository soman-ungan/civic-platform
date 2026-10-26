// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use axum::http::StatusCode;
use backend::AppState;
use common::{assert_no_key, request};
use serde_json::json;

async fn setup() -> (sqlx::SqlitePool, AppState) {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let state = common::test_state(pool.clone());
    (pool, state)
}

async fn get(state: &AppState, path: &str) -> common::TestResponse {
    request(state, "GET", path, None, None).await
}

#[tokio::test]
async fn lists_only_active_municipalities() {
    let (pool, state) = setup().await;
    let res = get(&state, "/api/municipalities").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json,
        json!([{ "id": "092100", "slug": "otawara", "name": "大田原市", "prefecture_name": "栃木県" }])
    );

    common::activate_second_municipality(&pool).await;
    let res = get(&state, "/api/municipalities").await;
    assert_eq!(res.json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn municipality_detail_has_only_its_districts() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;

    let res = get(&state, "/api/municipalities/otawara").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["name"], "大田原市");
    assert_eq!(
        res.json["districts"],
        json!([{ "id": "092100-kurobane", "name": "黒羽", "name_kana": "くろばね" }])
    );
}

#[tokio::test]
async fn inactive_or_unknown_municipality_is_404() {
    let (_pool, state) = setup().await;
    for path in [
        "/api/municipalities/nasushiobara",
        "/api/municipalities/nowhere",
        "/api/municipalities/nasushiobara/topics",
        "/api/municipalities/nasushiobara/topics/t2",
    ] {
        let res = get(&state, path).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(res.json["error"]["code"], "not_found", "{path}");
    }
}

#[tokio::test]
async fn topic_list_is_scoped_to_municipality() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;

    let otawara = get(&state, "/api/municipalities/otawara/topics").await;
    let ids: Vec<&str> = otawara.json["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["t1"]);

    let nasu = get(&state, "/api/municipalities/nasushiobara/topics").await;
    assert_eq!(nasu.json["topics"][0]["id"], "t2");
    assert_eq!(nasu.json["topics"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn topic_of_other_municipality_is_404() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;

    // 大田原市の URL で那須塩原市のトピックを指定する
    let res = get(&state, "/api/municipalities/otawara/topics/t2").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let res = get(&state, "/api/municipalities/nasushiobara/topics/t2").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["title"], "那須塩原のテスト");
}

#[tokio::test]
async fn topic_counts_only_approved_statements_and_distinct_voters() {
    let (pool, state) = setup().await;
    sqlx::raw_sql(
        "
        INSERT INTO statements (id, topic_id, body, status, created_at) VALUES
            ('s2', 't1', '承認済み1', 'approved', '2026-10-26T00:00:00Z'),
            ('s3', 't1', '承認済み2', 'approved', '2026-10-26T00:00:00Z'),
            ('s4', 't1', '却下', 'rejected', '2026-10-26T00:00:00Z');
        INSERT INTO votes (id, user_id, statement_id, vote_value, created_at) VALUES
            ('v1', 'u1', 's2', 1, '2026-10-26T00:00:00Z'),
            ('v2', 'u1', 's3', -1, '2026-10-26T00:00:00Z'),
            ('v3', 'u2', 's2', 0, '2026-10-26T00:00:00Z');
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    let res = get(&state, "/api/municipalities/otawara/topics/t1").await;
    assert_eq!(res.json["statement_count"], 2);
    assert_eq!(res.json["participant_count"], 2);
}

#[tokio::test]
async fn topics_can_be_filtered_by_tag() {
    let (pool, state) = setup().await;
    sqlx::raw_sql(
        "
        INSERT INTO topics (id, municipality_id, title, source_type, created_at) VALUES
            ('t3', '092100', '防災のテスト', 'assembly', '2026-10-27T00:00:00Z');
        INSERT INTO tags (id, kind, name) VALUES ('g-kosodate', 'field', '子育て'), ('g-bousai', 'field', '防災');
        INSERT INTO tags (id, kind, name, district_id) VALUES ('d-kurobane', 'district', '黒羽', '092100-kurobane');
        INSERT INTO topic_tags (topic_id, tag_id) VALUES
            ('t1', 'g-kosodate'), ('t1', 'd-kurobane'), ('t3', 'g-bousai'), ('t2', 'g-kosodate');
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    let all = get(&state, "/api/municipalities/otawara/topics").await;
    // 新しい順
    assert_eq!(all.json["topics"][0]["id"], "t3");
    assert_eq!(all.json["topics"][1]["id"], "t1");
    // t1 のタグ（kind, name 順）
    assert_eq!(
        all.json["topics"][1]["tags"],
        json!([
            { "id": "d-kurobane", "kind": "district", "name": "黒羽" },
            { "id": "g-kosodate", "kind": "field", "name": "子育て" }
        ])
    );
    // チップ用のタグは重複なしで、この自治体のトピックのものだけ
    let chip_ids: Vec<&str> = all.json["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(chip_ids, ["d-kurobane", "g-kosodate", "g-bousai"]);

    let filtered = get(&state, "/api/municipalities/otawara/topics?tag=g-kosodate").await;
    let ids: Vec<&str> = filtered.json["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    // 同じ分野タグが付いた那須塩原市の t2 は含まれない
    assert_eq!(ids, ["t1"]);
    // 絞り込んでもチップは全件のまま
    assert_eq!(filtered.json["tags"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn read_responses_have_no_user_id() {
    let (_pool, state) = setup().await;
    for path in [
        "/api/municipalities",
        "/api/municipalities/otawara",
        "/api/municipalities/otawara/topics",
        "/api/municipalities/otawara/topics/t1",
    ] {
        assert_no_key(&get(&state, path).await.json, "user_id");
    }
}
