// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use std::path::Path;

use backend::seed;
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
    let pool = common::test_pool().await;
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
    assert_eq!(districts, ["黒羽", "大田原", "湯津上"]);
}

#[tokio::test]
async fn seeds_are_idempotent() {
    let pool = common::test_pool().await;
    seed::run(&pool, seeds_dir()).await.unwrap();
    seed::run(&pool, seeds_dir()).await.unwrap();

    assert_eq!(count(&pool, "prefectures").await, 47);
    assert_eq!(count(&pool, "municipalities").await, 1);
    assert_eq!(count(&pool, "districts").await, 3);
}

fn dev_seeds_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/dev-seeds"))
}

#[tokio::test]
async fn dev_seeds_apply_after_master_seeds() {
    let pool = common::test_pool().await;
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

    // シード意見は投稿者なし・承認済み
    let not_seed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements WHERE user_id IS NOT NULL OR status <> 'approved'",
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
