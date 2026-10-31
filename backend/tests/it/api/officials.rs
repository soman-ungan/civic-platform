// SPDX-License-Identifier: AGPL-3.0-only

//! 議員の記録の API（ADR 0008）: 並びは首長が先・議員は読みの順（数では並べない）、公開済みのトピックだけ、
//! 自治体で絞る（別の自治体の議員は 404）、引用は新しい会議の順、トピックの出典に official_id

use crate::common::request;
use axum::http::StatusCode;
use backend::interface::api::AppState;

async fn setup() -> (sqlx::SqlitePool, AppState) {
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    crate::common::activate_second_municipality(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO public_officials (id, municipality_id, name, role, term_start, term_end, name_kana) VALUES
             ('o-mayor', '092100', '市長の人', 'mayor', '2022-03-20', NULL, 'んしちょう'),
             ('o-b', '092100', '議員B', 'councilor', '2023-11-19', NULL, 'いいいん'),
             ('o-a', '092100', '議員A', 'councilor', '2023-11-19', NULL, 'あいいん'),
             ('o-gone', '092100', '前の議員', 'councilor', '2023-11-19', '2026-02-20', 'ああ'),
             ('o-gone-quoted', '092100', '記録のある前の議員', 'councilor', '2023-11-19', '2026-02-20', 'かか'),
             ('o-nasu', '092134', '那須塩原の議員', 'councilor', '2023-11-19', NULL, 'なす');
         INSERT INTO topics (id, municipality_id, title, source_type, created_at, status) VALUES
             ('t-old', '092100', '前の会議のトピック', 'assembly', '2026-10-26T00:00:00Z', 'published'),
             ('t-new', '092100', '新しい会議のトピック', 'assembly', '2026-10-26T00:00:00Z', 'published'),
             ('t-draft', '092100', '下書き', 'assembly', '2026-10-26T00:00:00Z', 'draft');
         INSERT INTO tags (id, kind, name) VALUES ('g-kosodate', 'field', '子育て');
         INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t-new', 'g-kosodate');
         INSERT INTO documents (id, municipality_id, kind, title, held_on, url, sha256, char_count, imported_at) VALUES
             ('d1', '092100', 'minutes', '定例会（第2号）', '2026-03-01', 'https://example.jp/d1', 'h1', 1, '2026-10-26T00:00:00Z'),
             ('d2', '092100', 'minutes', '定例会（第3号）', '2026-06-17', 'https://example.jp/d2', 'h2', 1, '2026-10-26T00:00:00Z');
         INSERT INTO topic_sources (id, topic_id, document_id, utterance_no, speaker_label, official_id, quote, position) VALUES
             ('s1', 't-old', 'd1', 1, '議員A議員', 'o-a', '前の引用', 0),
             ('s2', 't-new', 'd2', 1, '議員A議員', 'o-a', '新しい引用1', 0),
             ('s3', 't-new', 'd2', 2, '市長', 'o-mayor', '市長の答弁', 1),
             ('s4', 't-new', 'd2', 3, '議員A議員', 'o-a', '新しい引用2', 2),
             ('s5', 't-draft', 'd2', 4, '議員A議員', 'o-a', '下書きの引用', 0),
             ('s6', 't-old', 'd1', 2, '記録のある前の議員議員', 'o-gone-quoted', '前の議員の引用', 1);",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = crate::common::test_state(pool.clone());
    (pool, state)
}

#[tokio::test]
async fn the_list_puts_the_mayor_first_then_councilors_by_reading() {
    let (_pool, state) = setup().await;
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/officials",
        None,
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
    let ids: Vec<&str> = res.json["officials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap())
        .collect();
    // 数では並べない（議員A が多くても読みの順）。任期の終わった人は記録のある人だけ。別の自治体の人は出ない
    assert_eq!(ids, ["o-mayor", "o-a", "o-b", "o-gone-quoted"]);
    // 下書きのトピックは数えない
    assert_eq!(res.json["officials"][1]["topic_count"], 2);
    assert_eq!(res.json["officials"][2]["topic_count"], 0);
    assert_eq!(
        res.json["coverage"],
        serde_json::json!({"from": "2026-03-01", "to": "2026-06-17"})
    );
    // 読みは並べ替えにだけ使い、返さない
    assert!(res.json["officials"][0].get("name_kana").is_none());
}

#[tokio::test]
async fn the_detail_lists_published_topics_with_the_officials_quotes() {
    let (_pool, state) = setup().await;
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/officials/o-a",
        None,
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
    assert_eq!(res.json["name"], "議員A");
    let topics = res.json["topics"].as_array().unwrap();
    let ids: Vec<&str> = topics.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["t-new", "t-old"], "新しい会議の順。下書きは出さない");
    let quotes: Vec<&str> = topics[0]["quotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["quote"].as_str().unwrap())
        .collect();
    assert_eq!(quotes, ["新しい引用1", "新しい引用2"], "その人の引用だけ");
    assert_eq!(topics[0]["quotes"][0]["url"], "https://example.jp/d2");
    assert_eq!(topics[0]["tags"][0]["id"], "g-kosodate");
    assert_eq!(topics[1]["tags"], serde_json::json!([]));

    // 記録の無い議員も同じ形
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/officials/o-b",
        None,
        None,
    )
    .await;
    assert_eq!(res.json["topics"], serde_json::json!([]));
}

#[tokio::test]
async fn officials_of_another_municipality_are_not_found() {
    let (_pool, state) = setup().await;
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/officials/o-nasu",
        None,
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = request(
        &state,
        "GET",
        "/api/municipalities/nasushiobara/officials",
        None,
        None,
    )
    .await;
    assert_eq!(res.json["officials"].as_array().unwrap().len(), 1);
    assert!(res.json["coverage"].is_null());
}

#[tokio::test]
async fn topic_sources_link_to_officials() {
    let (_pool, state) = setup().await;
    let res = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics/t-new",
        None,
        None,
    )
    .await;
    let ids: Vec<&serde_json::Value> = res.json["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["official_id"])
        .collect();
    assert_eq!(ids, ["o-a", "o-mayor", "o-a"]);
}
