// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use axum::http::StatusCode;
use backend::analysis::store::run_topic;
use backend::interface::api::AppState;
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
    let res = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json, json!({ "status": "pending" }));
}

#[tokio::test]
async fn ready_analysis_has_groups_consensus_and_map() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();

    let res = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await;
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
    // v00（賛成派）の位置とグループが返る
    assert!(j["map"]["you"]["group_id"].is_number());

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

/// fixture の参加者に関わり方を登録する
async fn add_relations(pool: &SqlitePool, rows: &[(&str, &str)]) {
    for (uid, rtype) in rows {
        sqlx::query(
            "INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at)
             VALUES ($1, '092100', $2, '2026-10-26T00:00:00Z')",
        )
        .bind(uid)
        .bind(rtype)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn residency_cells_use_secondary_suppression() {
    let (pool, state) = setup().await;
    // 住民 6 人、周辺 5 人、通勤・通学 3 人
    add_relations(
        &pool,
        &[
            ("v00", "resident"),
            ("v01", "resident"),
            ("v02", "resident"),
            ("v03", "resident"),
            ("v04", "resident"),
            ("v05", "resident"),
            ("v06", "neighbor"),
            ("v07", "neighbor"),
            ("v08", "neighbor"),
            ("v09", "neighbor"),
            ("v10", "neighbor"),
            ("v11", "commuter"),
            ("v12", "commuter"),
            ("v13", "commuter"),
        ],
    )
    .await;
    run_topic(&pool, "t1", false).await.unwrap();

    let j = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await
    .json;
    let rows = j["residency"].as_array().unwrap();
    let common = rows.iter().find(|r| r["statement_id"] == "common").unwrap();
    assert_eq!(
        common["by_type"]["resident"],
        json!({ "agree": 6, "seen": 6 })
    );
    // 通勤・通学（3人）を隠すだけだと、合計から逆算できる。次に少ない周辺（5人）も隠す
    assert_eq!(common["by_type"]["commuter"], Value::Null);
    assert_eq!(common["by_type"]["neighbor"], Value::Null);
    assert_eq!(common["by_type"]["related"], Value::Null);
}

#[tokio::test]
async fn group_composition_shows_relations_per_group() {
    let (pool, state) = setup().await;
    // 賛成派 v00〜v09 は住民 7・関係人口 3、反対派 v10〜v19 は住民 5・通勤・通学 5
    add_relations(
        &pool,
        &[
            ("v00", "resident"),
            ("v01", "resident"),
            ("v02", "resident"),
            ("v03", "resident"),
            ("v04", "resident"),
            ("v05", "resident"),
            ("v06", "resident"),
            ("v07", "related"),
            ("v08", "related"),
            ("v09", "related"),
            ("v10", "resident"),
            ("v11", "resident"),
            ("v12", "resident"),
            ("v13", "resident"),
            ("v14", "resident"),
            ("v15", "commuter"),
            ("v16", "commuter"),
            ("v17", "commuter"),
            ("v18", "commuter"),
            ("v19", "commuter"),
        ],
    )
    .await;
    run_topic(&pool, "t1", false).await.unwrap();

    let j = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await
    .json;
    let comps = j["group_composition"].as_array().unwrap();
    assert_eq!(comps.len(), 2);
    // v00 のグループ（賛成派）: 関係人口 3人は隠れ、その合計 3 は逆算されうるので住民 7人も隠れる
    let mine = j["map"]["you"]["group_id"].clone();
    let pro = comps.iter().find(|c| c["group_id"] == mine).unwrap();
    assert_eq!(
        pro["by_type"],
        json!({ "resident": null, "neighbor": null, "commuter": null, "related": null })
    );
    // 反対派: 住民 5・通勤・通学 5 はどちらも 5人以上なので見える
    let con = comps.iter().find(|c| c["group_id"] != mine).unwrap();
    assert_eq!(
        con["by_type"],
        json!({ "resident": 5, "neighbor": null, "commuter": 5, "related": null })
    );
    assert_no_key(&j, "user_id");
    assert_no_key(&j, "members");
}

#[tokio::test]
async fn too_few_participants_is_insufficient() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let state = common::test_state(pool.clone());
    run_topic(&pool, "t1", false).await.unwrap();
    let j = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await
    .json;
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
    // Cookie が無い人には何も返さない（投票前に全体の傾向を見せないため）
    let anonymous = get(&state, "/api/municipalities/otawara/consensus", None).await;
    assert_eq!(anonymous.json, json!([]));

    let v00 = common::cookie_for(&state, "v00");
    let res = get(&state, "/api/municipalities/otawara/consensus", Some(&v00)).await;
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
    let nasu = get(
        &state,
        "/api/municipalities/nasushiobara/consensus",
        Some(&v00),
    )
    .await;
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
    let j = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await
    .json;
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

    let j = get(
        &state,
        &format!("{T1}/analysis"),
        Some(&common::cookie_for(&state, "v00")),
    )
    .await
    .json;
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

#[tokio::test]
async fn analysis_is_locked_until_every_statement_is_voted() {
    let (pool, state) = setup().await;
    run_topic(&pool, "t1", false).await.unwrap();

    // Cookie が無い人
    let j = get(&state, &format!("{T1}/analysis"), None).await.json;
    assert_eq!(j, json!({ "status": "locked", "remaining": 7, "total": 7 }));

    // 一部だけ投票した人
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
    let vote = |sid: &'static str| {
        let (state, cookie) = (state.clone(), cookie.clone());
        async move {
            request(
                &state,
                "POST",
                &format!("{T1}/statements/{sid}/votes"),
                Some(&cookie),
                Some(json!({ "value": 0 })),
            )
            .await
        }
    };
    for sid in ["a0", "a1", "a2", "a3", "a4", "a5"] {
        vote(sid).await;
    }
    let j = get(&state, &format!("{T1}/analysis"), Some(&cookie))
        .await
        .json;
    assert_eq!(j, json!({ "status": "locked", "remaining": 1, "total": 7 }));
    // ロック中は合意点も返らない
    let pins = get(
        &state,
        "/api/municipalities/otawara/consensus",
        Some(&cookie),
    )
    .await
    .json;
    assert_eq!(pins, json!([]));

    // 最後の1件に投票すると見られる（パスでもよい）
    vote("common").await;
    let j = get(&state, &format!("{T1}/analysis"), Some(&cookie))
        .await
        .json;
    assert_eq!(j["status"], "ready");
    let pins = get(
        &state,
        "/api/municipalities/otawara/consensus",
        Some(&cookie),
    )
    .await
    .json;
    assert_eq!(pins.as_array().unwrap().len(), 1);

    // 意見が増えたら、また投票が必要になる
    sqlx::query("INSERT INTO statements (id, topic_id, body, status, created_at) VALUES ('new', 't1', '新しい意見', 'approved', '2026-10-27T00:00:00Z')")
        .execute(&pool)
        .await
        .unwrap();
    let j = get(&state, &format!("{T1}/analysis"), Some(&cookie))
        .await
        .json;
    assert_eq!(j, json!({ "status": "locked", "remaining": 1, "total": 8 }));
}
