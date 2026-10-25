mod common;

use sqlx::SqlitePool;

const NOW: &str = "2026-10-25T00:00:00Z";

/// topic / user / statement を 1 件ずつ入れる。
async fn seed(pool: &SqlitePool) {
    sqlx::query("INSERT INTO users (id, created_at) VALUES ('u1', ?)")
        .bind(NOW)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO topics (id, title, source_type, created_at) VALUES ('t1', 'テスト', 'citizen', ?)")
        .bind(NOW)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO statements (id, topic_id, user_id, body, created_at) VALUES ('s1', 't1', 'u1', '意見', ?)")
        .bind(NOW)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_statement(
    pool: &SqlitePool,
    id: &str,
    body: &str,
    status: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO statements (id, topic_id, body, status, created_at) VALUES (?, 't1', ?, ?, ?)",
    )
    .bind(id)
    .bind(body)
    .bind(status)
    .bind(NOW)
    .execute(pool)
    .await
    .map(|_| ())
}

async fn insert_vote(
    pool: &SqlitePool,
    id: &str,
    statement_id: &str,
    value: i64,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO votes (id, user_id, statement_id, vote_value, created_at) VALUES (?, 'u1', ?, ?, ?)")
        .bind(id)
        .bind(statement_id)
        .bind(value)
        .bind(NOW)
        .execute(pool)
        .await
        .map(|_| ())
}

#[tokio::test]
async fn core_tables_exist() {
    let pool = common::test_pool().await;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'topics', 'statements', 'votes') ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(tables, ["statements", "topics", "users", "votes"]);
}

#[tokio::test]
async fn statement_defaults_to_pending() {
    let pool = common::test_pool().await;
    seed(&pool).await;
    let status: String = sqlx::query_scalar("SELECT status FROM statements WHERE id = 's1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
}

#[tokio::test]
async fn statement_body_is_limited_to_140_chars() {
    let pool = common::test_pool().await;
    seed(&pool).await;
    // マルチバイト文字でバイト数ではなく文字数で数えていることを確認する
    assert!(
        insert_statement(&pool, "ok", &"あ".repeat(140), "approved")
            .await
            .is_ok()
    );
    assert!(
        insert_statement(&pool, "long", &"あ".repeat(141), "approved")
            .await
            .is_err()
    );
    assert!(
        insert_statement(&pool, "empty", "", "approved")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn statement_status_is_restricted() {
    let pool = common::test_pool().await;
    seed(&pool).await;
    assert!(insert_statement(&pool, "bad", "意見", "foo").await.is_err());
}

#[tokio::test]
async fn statement_requires_existing_topic() {
    let pool = common::test_pool().await;
    let res = sqlx::query("INSERT INTO statements (id, topic_id, body, created_at) VALUES ('s', 'missing', '意見', ?)")
        .bind(NOW)
        .execute(&pool)
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn vote_is_unique_per_user_and_statement() {
    let pool = common::test_pool().await;
    seed(&pool).await;
    assert!(insert_vote(&pool, "v1", "s1", 1).await.is_ok());
    assert!(insert_vote(&pool, "v2", "s1", -1).await.is_err());
}

#[tokio::test]
async fn vote_value_is_restricted() {
    let pool = common::test_pool().await;
    seed(&pool).await;
    assert!(insert_vote(&pool, "v1", "s1", 2).await.is_err());
}
