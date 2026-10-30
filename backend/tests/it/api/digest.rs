// SPDX-License-Identifier: AGPL-3.0-only

//! トピックマップと市民のまとめ（ADR 0005「市民のまとめ」）: 締め切ったトピックだけを地図に載せ、タグで絞る。
//! まとめは起点の近傍だけ、5人未満のトピックは中身を出さない、2自治体を混ぜない。
//! まとめから提案すると元のトピックが残り、公開されたら親子をたどれる

use axum::http::StatusCode;
use backend::interface::api::AppState;
use backend::process::analysis::store::run_topic;
use serde_json::json;

use crate::common::{
    activate_second_municipality, assert_no_key, cookie_for, request, seed_polarized, test_pool,
    test_state,
};

const OTAWARA: &str = "/api/municipalities/otawara";
const PAST: &str = "2026-01-01T00:00:00.000Z";

/// t1（2グループに分かれた投票）を路線バスの話題にし、似た t3・似ていない t4 と一緒に締め切る。
/// t5 は似ているが受付中。t2 は那須塩原市（似た題で締め切り）。どれも交通のタグ
async fn setup() -> AppState {
    let pool = test_pool().await;
    seed_polarized(&pool).await;
    activate_second_municipality(&pool).await;
    run_topic(&pool, "t1", false).await.unwrap();
    sqlx::raw_sql(&format!(
        "INSERT INTO tags (id, kind, name) VALUES ('field-koutsu', 'field', '交通') ON CONFLICT DO NOTHING;
         UPDATE topics SET title = '路線バスの本数を増やすべきか', description = '通学や通院のための路線バスの便を増やす',
                closes_at = '{PAST}' WHERE id IN ('t1', 't2');
         INSERT INTO topics (id, municipality_id, title, description, source_type, created_at, closes_at) VALUES
            ('t3', '092100', '通学の路線バスの便を増やすべきか', '高校生の通学のために路線バスの本数を増やす', 'citizen', '{PAST}', '{PAST}'),
            ('t4', '092100', '図書館の開館時間', '夜まで開ける', 'citizen', '{PAST}', '{PAST}'),
            ('t5', '092100', '路線バスの本数を増やすべきか', '通学や通院のための路線バスの便を増やす', 'citizen', '{PAST}', '2099-01-01T00:00:00.000Z');
         INSERT INTO topic_tags (topic_id, tag_id) VALUES
            ('t1', 'field-koutsu'), ('t2', 'field-koutsu'), ('t3', 'field-koutsu'), ('t4', 'field-koutsu'), ('t5', 'field-koutsu');
         UPDATE users SET registered_at = '{PAST}';
         UPDATE users SET is_operator = TRUE WHERE id = 'u1';
         INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at) VALUES
             ('u2', '092100', 'resident', '{PAST}');"
    ))
    .execute(&pool)
    .await
    .unwrap();
    test_state(pool)
}

#[tokio::test]
async fn map_shows_closed_topics_of_the_tag() {
    let state = setup().await;
    let res = request(&state, "GET", &format!("{OTAWARA}/topic-map"), None, None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["tags"][0]["id"], "field-koutsu");
    assert_eq!(res.json["tags"][0]["count"], 3);
    assert!(res.json["points"].as_array().unwrap().is_empty());

    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topic-map?tag=field-koutsu"),
        None,
        None,
    )
    .await;
    let mut ids: Vec<&str> = res.json["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    ids.sort();
    // 受付中の t5 と、那須塩原市の t2 は載らない
    assert_eq!(ids, ["t1", "t3", "t4"]);
    let edges = res.json["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(
        [
            edges[0]["a"].as_str().unwrap(),
            edges[0]["b"].as_str().unwrap()
        ],
        ["t1", "t3"]
    );
    // 同じ入力なら同じ配置
    let again = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topic-map?tag=field-koutsu"),
        None,
        None,
    )
    .await;
    assert_eq!(res.json, again.json);
}

#[tokio::test]
async fn digest_is_limited_to_the_neighborhood() {
    let state = setup().await;
    let path = |q: &str| format!("{OTAWARA}/digest?{q}");

    let res = request(&state, "GET", &path("anchor=t1&topics=t3"), None, None).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let topics = res.json["topics"].as_array().unwrap();
    assert_eq!(topics[0]["id"], "t1");
    assert_eq!(topics[0]["sufficient"], true);
    assert_eq!(topics[0]["consensus"][0]["statement_id"], "common");
    assert_eq!(topics[0]["divided"].as_array().unwrap().len(), 6);
    assert_eq!(topics[0]["groups"].as_array().unwrap().len(), 2);
    // t3 は投票が無いので、中身を出さない
    assert_eq!(topics[1]["sufficient"], false);
    assert!(topics[1]["consensus"].as_array().unwrap().is_empty());
    assert_no_key(&res.json, "user_id");

    // 近傍の外・受付中・別の自治体・起点が無い
    for q in [
        "anchor=t1&topics=t4",
        "anchor=t1&topics=t5",
        "anchor=t1&topics=t2",
        "anchor=t2",
        "anchor=t5",
    ] {
        let res = request(&state, "GET", &path(q), None, None).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{q}");
    }
}

#[tokio::test]
async fn proposals_from_a_digest_keep_their_parents() {
    let state = setup().await;
    let u2 = cookie_for(&state, "u2");
    let op = cookie_for(&state, "u1");
    let body = |parents: &[&str]| {
        json!({
            "title": "通学のバスを増やすには",
            "description": "前のトピックで合意した点をもとに、通学のバスの増やし方を考える。",
            "statements": ["全員賛成"],
            "parent_ids": parents,
        })
    };

    // 近傍の外は選べない（週1件にも数えない）
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/proposals"),
        Some(&u2),
        Some(body(&["t1", "t4"])),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/proposals"),
        Some(&u2),
        Some(body(&["t1", "t3"])),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
    let id = res.json["id"].as_str().unwrap().to_owned();

    // 運営者は下書きで元のトピックを見られる
    let res = request(
        &state,
        "GET",
        &format!("/api/operator/municipalities/otawara/topics/{id}"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["parents"][0]["id"], "t1");
    assert_eq!(res.json["parents"][1]["id"], "t3");

    // 週1件に数える
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/proposals"),
        Some(&u2),
        Some(body(&["t1"])),
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);

    // 公開するまで、元のトピックの画面に子は出ない
    let res = request(&state, "GET", &format!("{OTAWARA}/topics/t1"), None, None).await;
    assert!(res.json["children"].as_array().unwrap().is_empty());
    let res = request(
        &state,
        "POST",
        &format!("/api/operator/municipalities/otawara/topics/{id}/publish"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let res = request(&state, "GET", &format!("{OTAWARA}/topics/t1"), None, None).await;
    assert_eq!(res.json["children"][0]["id"], id);
    let res = request(&state, "GET", &format!("{OTAWARA}/topics/{id}"), None, None).await;
    assert_eq!(res.json["parents"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn consensus_page_shows_closed_topics_to_everyone_and_focuses_the_map() {
    let state = setup().await;
    // ピン留め（今までどおり）: Cookie が無ければ空
    let res = request(&state, "GET", &format!("{OTAWARA}/consensus"), None, None).await;
    assert!(res.json.as_array().unwrap().is_empty());
    // 合意点のページ: 締め切ったトピックの合意点は、投票していなくても出る
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/consensus?include_closed=true"),
        None,
        None,
    )
    .await;
    let items = res.json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["statement_id"], "common");
    assert_eq!(items[0]["closed"], true);

    // 合意点から地図へ: タグを渡さなくても、そのトピックの載る地図を開く
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topic-map?focus=t1"),
        None,
        None,
    )
    .await;
    assert_eq!(res.json["tag"], "field-koutsu");
    assert!(
        res.json["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "t1")
    );
    // 地図に載らないトピック（受付中）なら、タグの一覧だけ
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topic-map?focus=t5"),
        None,
        None,
    )
    .await;
    assert!(res.json["tag"].is_null());
}
