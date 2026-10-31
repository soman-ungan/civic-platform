// SPDX-License-Identifier: AGPL-3.0-only

//! 運営者の API（フェーズ6c）: 運営者でなければ 404、別の自治体のものは操作できない、
//! 投稿者の ID は返さない。中身は CLI と同じ `records`・`process` の関数

use crate::common::{assert_no_key, cookie_for, request, seed_fixture, test_pool, test_state};
use axum::http::StatusCode;
use serde_json::json;
use sqlx::SqlitePool;

const OTAWARA: &str = "/api/operator/municipalities/otawara";
const NASU: &str = "/api/operator/municipalities/nasushiobara";

/// u1 を運営者にし、t1・t2 を下書きにする
async fn setup(pool: &SqlitePool) {
    seed_fixture(pool).await;
    sqlx::raw_sql(
        "UPDATE users SET is_operator = TRUE, registered_at = '2026-10-26T00:00:00Z' WHERE id = 'u1';
         UPDATE topics SET status = 'draft';",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn only_operators_can_see_it() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let citizen = cookie_for(&state, "u2");
    for cookie in [None, Some(citizen.as_str())] {
        for path in [
            "/api/operator/municipalities".to_owned(),
            format!("{OTAWARA}/topics"),
            format!("{OTAWARA}/statements"),
        ] {
            let res = request(&state, "GET", &path, cookie, None).await;
            assert_eq!(res.status, StatusCode::NOT_FOUND, "{path}");
        }
        let res = request(
            &state,
            "POST",
            &format!("{OTAWARA}/topics/t1/publish"),
            cookie,
            None,
        )
        .await;
        assert_eq!(res.status, StatusCode::NOT_FOUND);
    }
    let status: String = sqlx::query_scalar("SELECT status FROM topics WHERE id = 't1'")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(status, "draft");
}

#[tokio::test]
async fn drafts_are_listed_and_published_per_municipality() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");

    // 準備中の自治体（那須塩原市は is_active = FALSE）も扱える
    let res = request(
        &state,
        "GET",
        "/api/operator/municipalities",
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json.as_array().unwrap().len(), 2);

    let res = request(&state, "GET", &format!("{OTAWARA}/topics"), Some(&op), None).await;
    assert_eq!(res.status, StatusCode::OK);
    let ids: Vec<&str> = res.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["t1"]);

    // 別の自治体のトピックは見えず、操作もできない
    for (method, path) in [
        ("GET", format!("{OTAWARA}/topics/t2")),
        ("POST", format!("{OTAWARA}/topics/t2/publish")),
        ("PATCH", format!("{OTAWARA}/topics/t2")),
        ("POST", format!("{OTAWARA}/topics/t2/statements")),
    ] {
        let res = request(
            &state,
            method,
            &path,
            Some(&op),
            Some(json!({"title": "x", "body": "x"})),
        )
        .await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{method} {path}");
    }

    // 手直ししてから公開すると、期限が付く
    let res = request(
        &state,
        "PATCH",
        &format!("{OTAWARA}/topics/t1"),
        Some(&op),
        Some(json!({"title": "直したタイトル"})),
    )
    .await;
    assert_eq!(res.json["title"], "直したタイトル");
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/publish"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["status"], "published");
    assert!(res.json["published_at"].is_string());
    assert!(res.json["closes_at"].is_string());
    assert_eq!(res.json["closed"], false);

    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics?status=published"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["items"].as_array().unwrap().len(), 1);
    // 公開中のトピックの様子（投票した人の数。誰かは返さない）
    sqlx::query(
        "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at)
         VALUES ('v-op', 'u2', 's1', 1, '2026-10-31T00:00:00Z')",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics?status=published"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["items"][0]["participant_count"], 1);
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics?status=closed"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let t2: String = sqlx::query_scalar("SELECT status FROM topics WHERE id = 't2'")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(t2, "draft");
}

#[tokio::test]
async fn deadlines_can_be_changed_only_for_published_topics() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");
    let path = format!("{OTAWARA}/topics/t1/closes-at");

    let res = request(&state, "PUT", &path, Some(&op), Some(json!({}))).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "not_published");

    request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/publish"),
        Some(&op),
        None,
    )
    .await;
    let res = request(
        &state,
        "PUT",
        &path,
        Some(&op),
        Some(json!({"closes_at": "2099-12-01T23:59:59+09:00"})),
    )
    .await;
    assert_eq!(res.json["closes_at"], "2099-12-01T14:59:59.000Z");
    assert_eq!(res.json["closed"], false);
    let res = request(
        &state,
        "PUT",
        &path,
        Some(&op),
        Some(json!({"closes_at": "12月1日"})),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // 期限を省くと、すぐ締め切る
    let res = request(&state, "PUT", &path, Some(&op), Some(json!({}))).await;
    assert_eq!(res.json["closed"], true);
}

#[tokio::test]
async fn statements_and_tags_of_drafts_can_be_edited() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");

    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/statements"),
        Some(&op),
        Some(json!({"body": "足した意見"})),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let added = res.json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["body"] == "足した意見")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/statements"),
        Some(&op),
        Some(json!({"body": ""})),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    let res = request(
        &state,
        "PATCH",
        &format!("{OTAWARA}/statements/{added}"),
        Some(&op),
        Some(json!({"body": "直した意見"})),
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    // 別の自治体の slug では操作できない
    let res = request(
        &state,
        "POST",
        &format!("{NASU}/statements/{added}/reject"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/statements/{added}/reject"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics/t1"),
        Some(&op),
        None,
    )
    .await;
    let bodies: Vec<&str> = res.json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["body"].as_str().unwrap())
        .collect();
    assert!(!bodies.contains(&"直した意見"));

    // タグ: 分野と、その自治体の地区だけ
    let res = request(&state, "GET", &format!("{OTAWARA}/tags"), Some(&op), None).await;
    let tag_ids: Vec<&str> = res
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert!(tag_ids.contains(&"district-092100-kurobane"));
    assert!(!tag_ids.contains(&"district-092134-shiobara"));
    let res = request(
        &state,
        "PUT",
        &format!("{OTAWARA}/topics/t1/tags/district-092134-shiobara"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let tag = format!("{OTAWARA}/topics/t1/tags/district-092100-kurobane");
    let res = request(&state, "PUT", &tag, Some(&op), None).await;
    assert_eq!(res.json["tags"][0]["assigned_by"], "admin");
    let res = request(&state, "DELETE", &tag, Some(&op), None).await;
    assert_eq!(res.json["tags"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn queue_hides_who_posted() {
    let pool = test_pool().await;
    setup(&pool).await;
    sqlx::raw_sql(
        "UPDATE topics SET status = 'published';
         UPDATE statements SET status = 'flagged', moderation_note = '人物への言及' WHERE id = 's1';
         INSERT INTO statements (id, topic_id, user_id, body, status, created_at) VALUES
             ('s2', 't2', 'u2', '那須塩原の意見', 'pending', '2026-10-26T00:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");

    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/statements"),
        Some(&op),
        None,
    )
    .await;
    let items = res.json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "s1");
    assert_eq!(items[0]["kind"], "citizen");
    assert_eq!(items[0]["topic_title"], "大田原のテスト");
    assert_eq!(items[0]["moderation_note"], "人物への言及");
    assert_no_key(&res.json, "user_id");
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics/t1"),
        Some(&op),
        None,
    )
    .await;
    assert_no_key(&res.json, "user_id");

    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/statements/s1/approve"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/statements"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["items"].as_array().unwrap().len(), 0);

    // キーが無い環境では、AI でのやり直しはできない（pending を勝手に承認しない）
    let res = request(
        &state,
        "POST",
        &format!("{NASU}/statements/s2/retry"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.json["error"]["code"], "llm_unavailable");
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/statements/s2/retry"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn lists_are_read_in_pages_without_skipping() {
    let pool = test_pool().await;
    setup(&pool).await;
    // 下書きを5件（t1 と、同じ日時の d1〜d4）
    for i in 1..=4 {
        sqlx::query(
            "INSERT INTO topics (id, municipality_id, title, source_type, created_at, status)
             VALUES ($1, '092100', $2, 'assembly', '2026-10-26T00:00:00Z', 'draft')",
        )
        .bind(format!("d{i}"))
        .bind(format!("下書き{i}"))
        .execute(&pool)
        .await
        .unwrap();
    }
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");
    let page = |after: Option<String>| {
        let state = state.clone();
        let op = op.clone();
        async move {
            let after = after.map(|a| format!("&after={a}")).unwrap_or_default();
            request(
                &state,
                "GET",
                &format!("{OTAWARA}/topics?limit=2{after}"),
                Some(&op),
                None,
            )
            .await
            .json
        }
    };

    let first = page(None).await;
    let ids: Vec<String> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids, ["d1", "d2"]);
    assert_eq!(first["next"], "d2");

    // 読んだものを公開して一覧から抜けても、続きは読み飛ばさない
    request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/d1/publish"),
        Some(&op),
        None,
    )
    .await;
    let second = page(Some("d2".into())).await;
    let ids: Vec<&str> = second["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["d3", "d4"]);
    let third = page(second["next"].as_str().map(str::to_owned)).await;
    let ids: Vec<&str> = third["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["t1"]);
    assert!(third["next"].is_null());

    // 公開中は新しい順（d1 だけ）
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics?status=published&limit=1"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["items"][0]["id"], "d1");
    assert!(res.json["next"].is_null());
}

#[tokio::test]
async fn drafts_are_queued_and_publish_slots_are_managed_per_municipality() {
    let pool = test_pool().await;
    setup(&pool).await;
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");

    // 公開待ちに入れる（下書きだけ）。別の自治体のトピックは 404
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/queue"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert!(res.json["queued_at"].is_string());
    assert_eq!(res.json["status"], "draft");
    // 下書きの一覧からは外れる（公開待ちの一覧に出る）
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics?status=draft"),
        Some(&op),
        None,
    )
    .await;
    assert!(!res.json.to_string().contains("\"t1\""));
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t2/queue"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    // 時刻が無ければ予定は無い
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/publishing"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["queue"][0]["id"], "t1");
    assert!(res.json["queue"][0]["planned_at"].is_null());

    // 時刻を足す・変える・消す。形が違う・同じ時刻は 422
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/publish-slots"),
        Some(&op),
        Some(json!({"time_of_day": "7:30"})),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["slots"][0]["time_of_day"], "07:30");
    assert!(res.json["queue"][0]["planned_at"].is_string());
    let slot = res.json["slots"][0]["id"].as_str().unwrap().to_owned();
    for bad in ["25:00", "7時", "07:30"] {
        let res = request(
            &state,
            "POST",
            &format!("{OTAWARA}/publish-slots"),
            Some(&op),
            Some(json!({"time_of_day": bad})),
        )
        .await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    let res = request(
        &state,
        "PATCH",
        &format!("{OTAWARA}/publish-slots/{slot}"),
        Some(&op),
        Some(json!({"time_of_day": "18:00"})),
    )
    .await;
    assert_eq!(res.json["slots"][0]["time_of_day"], "18:00");
    // 別の自治体の時刻としては消せない
    let res = request(
        &state,
        "DELETE",
        &format!("{NASU}/publish-slots/{slot}"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = request(
        &state,
        "GET",
        &format!("{NASU}/publishing"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["slots"].as_array().unwrap().len(), 0);
    assert_eq!(res.json["queue"].as_array().unwrap().len(), 0);
    let res = request(
        &state,
        "DELETE",
        &format!("{OTAWARA}/publish-slots/{slot}"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["slots"].as_array().unwrap().len(), 0);

    // 公開待ちから外す。公開すれば待ちから抜ける
    let res = request(
        &state,
        "DELETE",
        &format!("{OTAWARA}/topics/t1/queue"),
        Some(&op),
        None,
    )
    .await;
    assert!(res.json["queued_at"].is_null());
    request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/queue"),
        Some(&op),
        None,
    )
    .await;
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/publish"),
        Some(&op),
        None,
    )
    .await;
    assert!(res.json["queued_at"].is_null());
    let res = request(
        &state,
        "POST",
        &format!("{OTAWARA}/topics/t1/queue"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::CONFLICT);

    // 運営者でなければ 404
    let citizen = cookie_for(&state, "u2");
    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/publishing"),
        Some(&citizen),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn drafts_show_similar_topics_in_the_same_municipality() {
    let pool = test_pool().await;
    setup(&pool).await;
    sqlx::raw_sql(
        "UPDATE topics SET title = '路線バスを増便すべきか', description = '通学の足として路線バスの本数を増やす' WHERE id = 't1';
         UPDATE topics SET title = '路線バスを増便すべきか', description = '通学の足として路線バスの本数を増やす' WHERE id = 't2';
         INSERT INTO topics (id, municipality_id, title, description, source_type, status, created_at) VALUES
            ('t3', '092100', '路線バスの本数を増やすべきか', '通学のために路線バスを増便する', 'assembly', 'published', '2026-10-27T00:00:00Z'),
            ('t4', '092100', '図書館の開館時間', '夜まで開ける', 'assembly', 'published', '2026-10-27T00:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = test_state(pool);
    let op = cookie_for(&state, "u1");

    let res = request(&state, "GET", &format!("{OTAWARA}/topics"), Some(&op), None).await;
    let similar = &res.json["items"][0]["similar"];
    let ids: Vec<&str> = similar
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    // 別の自治体の t2 は同じ文でも出さない。似ていない t4 も出さない
    assert_eq!(ids, ["t3"]);
    assert!(similar[0]["score"].as_f64().unwrap() >= 0.25);

    let res = request(
        &state,
        "GET",
        &format!("{OTAWARA}/topics/t1"),
        Some(&op),
        None,
    )
    .await;
    assert_eq!(res.json["similar"][0]["id"], "t3");
}
