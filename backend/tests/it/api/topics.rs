// SPDX-License-Identifier: AGPL-3.0-only

//! 自治体とトピックの API: 公開中の自治体だけ、自治体で絞る（別の自治体のものは 404）、
//! 承認済みの意見と投票した人の数、タグの絞り込み、投稿者の ID を返さない、参加状態

use crate::common::{assert_no_key, request};
use axum::http::StatusCode;
use backend::interface::api::AppState;
use serde_json::json;

async fn setup() -> (sqlx::SqlitePool, AppState) {
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    let state = crate::common::test_state(pool.clone());
    (pool, state)
}

async fn get(state: &AppState, path: &str) -> crate::common::TestResponse {
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

    crate::common::activate_second_municipality(&pool).await;
    let res = get(&state, "/api/municipalities").await;
    assert_eq!(res.json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn municipality_detail_has_only_its_districts() {
    let (pool, state) = setup().await;
    crate::common::activate_second_municipality(&pool).await;

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
    crate::common::activate_second_municipality(&pool).await;

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
    crate::common::activate_second_municipality(&pool).await;

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

#[tokio::test]
async fn participation_status_follows_votes() {
    let (pool, state) = setup().await;
    // t1 に承認済みの意見を2件（fixture の s1 は未承認なので数えない）
    sqlx::raw_sql(
        "INSERT INTO statements (id, topic_id, body, status, created_at) VALUES
            ('a1', 't1', '意見1', 'approved', '2026-10-26T00:00:00Z'),
            ('a2', 't1', '意見2', 'approved', '2026-10-26T00:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let status = |j: &serde_json::Value| (j["participation"].clone(), j["voted_count"].clone());

    // Cookie が無ければ未参加
    let res = get(&state, "/api/municipalities/otawara/topics").await;
    assert_eq!(status(&res.json["topics"][0]), (json!("none"), json!(0)));

    // 居住地属性を登録しただけでは未参加のまま
    let cookie = request(
        &state,
        "PUT",
        "/api/municipalities/otawara/relation",
        None,
        Some(json!({ "residency_type": "resident" })),
    )
    .await
    .cookie
    .unwrap();
    let list = |cookie: String| {
        let state = state.clone();
        async move {
            request(
                &state,
                "GET",
                "/api/municipalities/otawara/topics",
                Some(&cookie),
                None,
            )
            .await
            .json
        }
    };
    assert_eq!(
        status(&list(cookie.clone()).await["topics"][0]),
        (json!("none"), json!(0))
    );

    let vote = |sid: &'static str| {
        let (state, cookie) = (state.clone(), cookie.clone());
        async move {
            let res = request(
                &state,
                "POST",
                &format!("/api/municipalities/otawara/topics/t1/statements/{sid}/votes"),
                Some(&cookie),
                Some(json!({ "value": 0 })),
            )
            .await;
            assert_eq!(res.status, StatusCode::OK);
        }
    };
    vote("a1").await;
    assert_eq!(
        status(&list(cookie.clone()).await["topics"][0]),
        (json!("in_progress"), json!(1))
    );
    vote("a2").await;
    assert_eq!(
        status(&list(cookie.clone()).await["topics"][0]),
        (json!("completed"), json!(2))
    );

    // 詳細でも同じ。意見が増えたら参加中に戻る
    let detail = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics/t1",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status(&detail.json), (json!("completed"), json!(2)));
    sqlx::query("INSERT INTO statements (id, topic_id, body, status, created_at) VALUES ('a3', 't1', '意見3', 'approved', '2026-10-26T00:00:00Z')")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        status(&list(cookie.clone()).await["topics"][0]),
        (json!("in_progress"), json!(2))
    );
}

#[tokio::test]
async fn topics_are_ordered_by_publish_date_with_new_marks_and_sessions() {
    let (pool, state) = setup().await;
    let recent = backend::shared::util::now_iso();
    // t1 は古く作って最近公開（新着）、t3 は新しく作って前に公開、t4 は期限の決まりより前（published_at なし）
    sqlx::raw_sql(&format!(
        "UPDATE topics SET source_type = 'assembly', published_at = '{recent}' WHERE id = 't1';
         INSERT INTO topics (id, municipality_id, title, source_type, created_at, published_at) VALUES
             ('t3', '092100', '前に公開', 'citizen', '2026-10-28T00:00:00.000Z', '2026-10-01T00:00:00.000Z'),
             ('t4', '092100', '期限の前', 'citizen', '2026-09-01T00:00:00.000Z', NULL);
         INSERT INTO documents (id, municipality_id, kind, title, held_on, sha256, char_count, imported_at) VALUES
             ('d2', '092100', 'minutes', '令和８年第２回大田原市議会定例会（第３号）', '2026-06-17', 'h2', 1, '2026-10-26T00:00:00Z'),
             ('d1', '092100', 'minutes', '令和８年第２回大田原市議会定例会（第２号）', '2026-06-16', 'h1', 1, '2026-10-26T00:00:00Z');
         INSERT INTO topic_sources (id, topic_id, document_id, utterance_no, speaker_label, quote, position) VALUES
             ('ts2', 't1', 'd2', 1, '市長', '引用2', 0),
             ('ts1', 't1', 'd1', 1, '市長', '引用1', 1);"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let res = get(&state, "/api/municipalities/otawara/topics").await;
    let topics = res.json["topics"].as_array().unwrap();
    let ids: Vec<&str> = topics.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["t1", "t3", "t4"]);
    assert_eq!(topics[0]["is_new"], true);
    assert_eq!(topics[1]["is_new"], false);
    // 会期は出典の最初の会議（6月16日）から。市民の提案には無い
    assert_eq!(topics[0]["session"], "令和8年 第2回定例会（6月）");
    assert!(topics[1]["session"].is_null());

    let res = get(&state, "/api/municipalities/otawara/topics/t1").await;
    assert_eq!(res.json["session"], "令和8年 第2回定例会（6月）");
}
