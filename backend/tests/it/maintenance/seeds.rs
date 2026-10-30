// SPDX-License-Identifier: AGPL-3.0-only

//! seed（`data/seeds/`・`data/dev-seeds/`）: マイグレーションに当たること、何度入れても同じこと、サンプルで分析が出ること

use std::path::Path;

use backend::maintenance::seed;
use sqlx::SqlitePool;

fn seeds_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/seeds"))
}

async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn seeds_apply_to_schema() {
    let pool = crate::common::test_pool().await;
    let files = seed::run(&pool, seeds_dir()).await.unwrap();
    assert!(!files.is_empty());

    assert_eq!(count(&pool, "prefectures").await, 47);

    let (name, slug, active): (String, String, bool) =
        sqlx::query_as("SELECT name, slug, is_active FROM municipalities WHERE id = '092100'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (name.as_str(), slug.as_str(), active),
        ("大田原市", "otawara", true)
    );

    let districts: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM districts WHERE municipality_id = '092100' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        districts,
        ["親園", "金田", "黒羽", "野崎", "大田原", "佐久山", "湯津上"]
    );
}

#[tokio::test]
async fn seeds_are_idempotent() {
    let pool = crate::common::test_pool().await;
    seed::run(&pool, seeds_dir()).await.unwrap();
    seed::run(&pool, seeds_dir()).await.unwrap();

    assert_eq!(count(&pool, "prefectures").await, 47);
    assert_eq!(count(&pool, "municipalities").await, 1);
    assert_eq!(count(&pool, "districts").await, 7);
}

fn dev_seeds_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/dev-seeds"))
}

#[tokio::test]
async fn dev_seeds_apply_after_master_seeds() {
    let pool = crate::common::test_pool().await;
    seed::run(&pool, seeds_dir()).await.unwrap();
    seed::run(&pool, dev_seeds_dir()).await.unwrap();
    // 冪等性
    seed::run(&pool, dev_seeds_dir()).await.unwrap();

    // サンプルのトピックは必ず【サンプル】と分かるようにする
    let titles: Vec<String> = sqlx::query_scalar("SELECT title FROM topics ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(!titles.is_empty());
    for title in &titles {
        assert!(title.starts_with("【サンプル】"), "{title}");
    }

    // シード意見は投稿者なし・承認済み・最初の意見
    let not_seed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements
         WHERE user_id IS NOT NULL OR status <> 'approved' OR NOT is_seed",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(not_seed, 0);

    // 地区タグはその自治体の地区だけを指す
    let foreign: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM topic_tags tt
         JOIN topics t ON t.id = tt.topic_id
         JOIN tags g ON g.id = tt.tag_id
         JOIN districts d ON d.id = g.district_id
         WHERE d.municipality_id <> t.municipality_id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(foreign, 0);
}

/// 開発用サンプルだけで、どのトピックでも分析（グループと合意点）が画面に出ること。
/// DB を初期化して `seed --dev` したあと、すぐに動作確認できることを保証する
#[tokio::test]
async fn dev_seeds_are_enough_for_analysis() {
    use axum::http::StatusCode;
    use backend::process::analysis::store::run_all;

    let pool = crate::common::test_pool().await;
    seed::run(&pool, seeds_dir()).await.unwrap();
    seed::run(&pool, dev_seeds_dir()).await.unwrap();
    assert_eq!(run_all(&pool, false).await.unwrap(), 5);

    let state = crate::common::test_state(pool.clone());
    let topics: Vec<String> = sqlx::query_scalar("SELECT id FROM topics ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    let mut completed_cookie = None;
    for topic in &topics {
        // 分析はすべての意見に投票した人にだけ見える。全件に投票したサンプル参加者で確かめる
        let voter: String = sqlx::query_scalar(
            "SELECT v.user_id FROM votes v JOIN statements s ON s.id = v.statement_id
             WHERE s.topic_id = $1 AND s.status = 'approved'
             GROUP BY v.user_id
             HAVING count(*) = (SELECT count(*) FROM statements WHERE topic_id = $1 AND status = 'approved')
             ORDER BY v.user_id LIMIT 1",
        )
        .bind(topic)
        .fetch_one(&pool)
        .await
        .unwrap();
        let cookie = crate::common::cookie_for(&state, &voter);
        let res = crate::common::request(
            &state,
            "GET",
            &format!("/api/municipalities/otawara/topics/{topic}/analysis"),
            Some(&cookie),
            None,
        )
        .await;
        completed_cookie = Some(cookie);
        assert_eq!(res.status, StatusCode::OK);
        let j = &res.json;
        assert_eq!(j["status"], "ready", "{topic}: {j}");
        assert!(
            j["groups"].as_array().unwrap().len() >= 2,
            "{topic}: グループが2つ以上"
        );
        assert!(
            !j["consensus"].as_array().unwrap().is_empty(),
            "{topic}: 合意点がある"
        );
        // 居住地属性の比較表に出せる意見がある（5人以上の区分が2つ以上）
        assert!(
            !j["residency"].as_array().unwrap().is_empty(),
            "{topic}: 居住地属性の集計がある"
        );
        crate::common::assert_no_key(j, "user_id");
    }

    // タイムラインのピン留めにも出る
    let pins = crate::common::request(
        &state,
        "GET",
        "/api/municipalities/otawara/consensus",
        completed_cookie.as_deref(),
        None,
    )
    .await;
    assert!(!pins.json.as_array().unwrap().is_empty());

    // 締め切ったサンプル（4・5）でトピックマップとまとめを試せる: 交通のタグの地図に載り、互いに近傍で、
    // 合意点と意見が分かれた点の両方が出る
    let map = backend::process::topic_map::topic_map(&pool, "092100", Some("field-koutsu"))
        .await
        .unwrap();
    let mut ids: Vec<&str> = map.points.iter().map(|p| p.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, ["sample-092100-4", "sample-092100-5"]);
    assert_eq!(map.edges.len(), 1);
    let digest = backend::process::digest::digest(
        &pool,
        "092100",
        "sample-092100-4",
        &["sample-092100-5".to_owned()],
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(digest.topics.len(), 2);
    for t in &digest.topics {
        assert!(
            t.sufficient && !t.consensus.is_empty() && !t.divided.is_empty(),
            "{}",
            t.id
        );
        assert!(t.groups.iter().all(|g| !g.representative.is_empty()));
        assert!(t.residency.values().any(Option::is_some));
    }
}
