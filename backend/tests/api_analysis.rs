// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use axum::http::StatusCode;
use backend::AppState;
use backend::analysis::store::run_topic;
use common::{assert_no_key, request};
use serde_json::{Value, json};
use sqlx::SqlitePool;

const T1: &str = "/api/municipalities/otawara/topics/t1";

async fn setup() -> (SqlitePool, AppState) {
    let pool = common::test_pool().await;
    common::seed_polarized(&pool).await;
    let state = common::test_state(pool.clone());
    (pool, state)
}

async fn get(state: &AppState, path: &str, cookie: Option<&str>) -> common::TestResponse {
    request(state, "GET", path, cookie, None).await
}

#[tokio::test]
async fn pending_before_first_computation() {
    let (_pool, state) = setup().await;
    let res = get(&state, &format!("{T1}/analysis"), None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json, json!({ "status": "pending" }));
}

#[tokio::test]
async fn ready_analysis_has_groups_consensus_and_map() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();

    let res = get(&state, &format!("{T1}/analysis"), None).await;
    assert_eq!(res.status, StatusCode::OK);
    let j = &res.json;
    assert_eq!(j["status"], "ready");
    assert_eq!(j["participant_count"], 20);
    assert_eq!(j["analyzed_count"], 20);

    // グループは A, B の2つ、各10人。代表意見がある
    let groups = j["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0]["label"], "A");
    assert_eq!(groups[1]["label"], "B");
    assert!(groups.iter().all(|g| g["size"] == 10));
    assert!(
        groups
            .iter()
            .all(|g| !g["representative"].as_array().unwrap().is_empty())
    );

    // 全員が賛成した意見だけがピン留めされる（両グループで 11/12 ≥ 0.8）
    let consensus = j["consensus"].as_array().unwrap();
    assert_eq!(consensus.len(), 1);
    assert_eq!(consensus[0]["statement_id"], "common");
    assert_eq!(consensus[0]["body"], "全員賛成");
    assert!((consensus[0]["gac"].as_f64().unwrap() - (11.0f64 / 12.0).powi(2)).abs() < 1e-12);
    assert_eq!(consensus[0]["groups"][0]["agree"], 10);

    // 意見マップ: 基底クラスタは中心と人数だけ。Cookie が無ければ you は null
    let base = j["map"]["base_clusters"].as_array().unwrap();
    let total: u64 = base.iter().map(|b| b["count"].as_u64().unwrap()).sum();
    assert_eq!(total, 20);
    assert!(base.iter().all(|b| b.get("members").is_none()));
    assert_eq!(j["map"]["statements"].as_array().unwrap().len(), 7);
    assert_eq!(j["map"]["you"], Value::Null);

    assert_no_key(j, "user_id");
    assert_no_key(j, "members");
}

#[tokio::test]
async fn you_are_placed_in_your_group() {
    let (pool, state) = setup().await;

    // 新しい参加者が API で居住地属性を登録し、賛成派（v00〜v09）と同じように投票する
    let res = request(
        &state,
        "PUT",
        "/api/municipalities/otawara/relation",
        None,
        Some(json!({ "residency_type": "resident" })),
    )
    .await;
    let cookie = res.cookie.unwrap();
    for sid in ["a0", "a1", "a2", "a3", "a4", "a5", "common"] {
        let res = request(
            &state,
            "POST",
            &format!("{T1}/statements/{sid}/votes"),
            Some(&cookie),
            Some(json!({ "value": 1 })),
        )
        .await;
        assert_eq!(res.status, StatusCode::OK);
    }
    run_topic(&pool, "t1", false).await.unwrap();

    let j = get(&state, &format!("{T1}/analysis"), Some(&cookie))
        .await
        .json;
    let you = &j["map"]["you"];
    assert!(you["x"].is_number() && you["y"].is_number());
    // 自分のグループは、賛成派（11人になった方）
    let my_group = j["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == you["group_id"])
        .expect("グループに入っている");
    assert_eq!(my_group["size"], 11);
    assert_no_key(&j, "user_id");
}

#[tokio::test]
async fn residency_cells_with_few_people_are_hidden() {
    let (pool, state) = setup().await;
    // 住民 6 人（v00〜v05）、通勤・通学 3 人（v10〜v12）
    for (uid, rtype) in [
        ("v00", "resident"),
        ("v01", "resident"),
        ("v02", "resident"),
        ("v03", "resident"),
        ("v04", "resident"),
        ("v05", "resident"),
        ("v10", "commuter"),
        ("v11", "commuter"),
        ("v12", "commuter"),
    ] {
        sqlx::query(
            "INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at)
             VALUES ($1, '092100', $2, '2026-10-26T00:00:00Z')",
        )
        .bind(uid)
        .bind(rtype)
        .execute(&pool)
        .await
        .unwrap();
    }
    run_topic(&pool, "t1", false).await.unwrap();

    let j = get(&state, &format!("{T1}/analysis"), None).await.json;
    let rows = j["residency"].as_array().unwrap();
    let common = rows.iter().find(|r| r["statement_id"] == "common").unwrap();
    assert_eq!(
        common["by_type"]["resident"],
        json!({ "agree": 6, "seen": 6 })
    );
    // 3 人しかいないセルは出さない
    assert_eq!(common["by_type"]["commuter"], Value::Null);
    assert_eq!(common["by_type"]["neighbor"], Value::Null);
}

#[tokio::test]
async fn too_few_participants_is_insufficient() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let state = common::test_state(pool.clone());
    run_topic(&pool, "t1", false).await.unwrap();
    let j = get(&state, &format!("{T1}/analysis"), None).await.json;
    assert_eq!(j["status"], "insufficient");
    assert_eq!(j["required"], 5);
}

#[tokio::test]
async fn analysis_of_other_municipality_topic_is_404() {
    let (pool, state) = setup().await;
    common::activate_second_municipality(&pool).await;
    let res = get(
        &state,
        "/api/municipalities/otawara/topics/t2/analysis",
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn municipality_consensus_lists_pinned_statements() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();
    let res = get(&state, "/api/municipalities/otawara/consensus", None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json,
        json!([{
            "topic_id": "t1",
            "topic_title": "大田原のテスト",
            "statement_id": "common",
            "body": "全員賛成",
            "gac": (11.0f64 / 12.0).powi(2),
        }])
    );
    // 別の自治体の合意点は混ざらない
    common::activate_second_municipality(&pool).await;
    let nasu = get(&state, "/api/municipalities/nasushiobara/consensus", None).await;
    assert_eq!(nasu.json, json!([]));
}

#[tokio::test]
async fn next_statement_still_returns_only_unvoted_approved() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();
    // 優先度付きになっても、新しい訪問者には承認済みの意見だけが出る
    for _ in 0..20 {
        let j = get(&state, &format!("{T1}/next-statement"), None)
            .await
            .json;
        let id = j["statement"]["id"].as_str().unwrap();
        assert!(
            ["a0", "a1", "a2", "a3", "a4", "a5", "common"].contains(&id),
            "{id}"
        );
    }
}

#[tokio::test]
async fn best_agree_is_shown_as_agree() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();
    let j = get(&state, &format!("{T1}/analysis"), None).await.json;
    // どのグループの代表意見にも「0人が反対」のような表示は出ない
    for g in j["groups"].as_array().unwrap() {
        for r in g["representative"].as_array().unwrap() {
            assert!(r["n_success"].as_f64().unwrap() > 0.0, "{r}");
        }
    }
}

#[tokio::test]
async fn statements_by_group_lists_every_approved_statement() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();
    // 分析の後に非公開になった意見は表に出さない
    sqlx::query("UPDATE statements SET status = 'rejected' WHERE id = 'a5'")
        .execute(&pool)
        .await
        .unwrap();

    let j = get(&state, &format!("{T1}/analysis"), None).await.json;
    let rows = j["statements_by_group"].as_array().unwrap();
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r["statement_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["a0", "a1", "a2", "a3", "a4", "common"]);

    // 各行にグループの数だけ列があり、グループの id はグループ一覧と同じ順
    let group_ids: Vec<&Value> = j["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| &g["id"])
        .collect();
    for row in rows {
        let cols: Vec<&Value> = row["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| &c["group_id"])
            .collect();
        assert_eq!(cols, group_ids);
    }
    // 全員が賛成した意見は、どのグループでも 10 人中 10 人が賛成
    let common = rows.iter().find(|r| r["statement_id"] == "common").unwrap();
    for c in common["groups"].as_array().unwrap() {
        assert_eq!(
            (
                c["agree"].as_u64(),
                c["disagree"].as_u64(),
                c["seen"].as_u64()
            ),
            (Some(10), Some(0), Some(10))
        );
    }
    // a0 は一方のグループが全員賛成、もう一方が全員反対
    let a0 = rows.iter().find(|r| r["statement_id"] == "a0").unwrap();
    let mut agrees: Vec<u64> = a0["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["agree"].as_u64().unwrap())
        .collect();
    agrees.sort();
    assert_eq!(agrees, [0, 10]);
    assert_no_key(&j, "user_id");
}
