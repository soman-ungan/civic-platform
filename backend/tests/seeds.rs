// SPDX-License-Identifier: AGPL-3.0-or-later

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
