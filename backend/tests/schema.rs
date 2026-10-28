// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use sqlx::SqlitePool;

const NOW: &str = "2026-10-26T00:00:00Z";

async fn exec(pool: &SqlitePool, sql: &str) -> sqlx::Result<()> {
    sqlx::query(sql).execute(pool).await.map(|_| ())
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

async fn insert_relation(
    pool: &SqlitePool,
    municipality_id: &str,
    residency_type: &str,
    district_id: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, district_id, created_at) VALUES ('u1', ?, ?, ?, ?)",
    )
    .bind(municipality_id)
    .bind(residency_type)
    .bind(district_id)
    .bind(NOW)
    .execute(pool)
    .await
    .map(|_| ())
}

#[tokio::test]
async fn all_tables_exist() {
    let pool = common::test_pool().await;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE '\\_%' ESCAPE '\\' AND name NOT LIKE 'sqlite%' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        tables,
        [
            "budget_items",
            "budget_project_tags",
            "budget_projects",
            "budgets",
            "collector_sources",
            "districts",
            "documents",
            "fetch_cache",
            "llm_usage",
            "municipalities",
            "municipality_stats",
            "operator_invites",
            "passkeys",
            "prefectures",
            "public_officials",
            "settlement_item_tags",
            "settlement_items",
            "statement_sources",
            "statements",
            "tags",
            "topic_analyses",
            "topic_sources",
            "topic_tags",
            "topics",
            "user_municipality_relations",
            "users",
            "votes",
            "webauthn_challenges",
        ]
    );
}

// ===== 行政区分 =====

#[tokio::test]
async fn municipality_slug_is_unique() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let res = exec(
        &pool,
        "INSERT INTO municipalities (id, prefecture_code, name, name_kana, slug, created_at) VALUES ('092011', '09', '宇都宮市', 'うつのみやし', 'otawara', '2026-10-26T00:00:00Z')",
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn municipality_requires_existing_prefecture() {
    let pool = common::test_pool().await;
    let res = exec(
        &pool,
        "INSERT INTO municipalities (id, prefecture_code, name, name_kana, slug, created_at) VALUES ('092100', '09', '大田原市', 'おおたわらし', 'otawara', '2026-10-26T00:00:00Z')",
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn district_name_is_unique_within_municipality_only() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    // 同じ自治体に同名の地区は入らない
    let dup = exec(
        &pool,
        "INSERT INTO districts (id, municipality_id, name, name_kana) VALUES ('092100-kurobane2', '092100', '黒羽', 'くろばね')",
    )
    .await;
    assert!(dup.is_err());
    // 別の自治体なら同名でもよい
    let other = exec(
        &pool,
        "INSERT INTO districts (id, municipality_id, name, name_kana) VALUES ('092134-kurobane', '092134', '黒羽', 'くろばね')",
    )
    .await;
    assert!(other.is_ok());
}

// ===== ユーザーと自治体の関係 =====

#[tokio::test]
async fn relation_accepts_district_of_same_municipality() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    assert!(
        insert_relation(&pool, "092100", "resident", Some("092100-kurobane"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn relation_rejects_district_of_other_municipality() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    // 大田原市との関係に那須塩原市の地区は指定できない
    assert!(
        insert_relation(&pool, "092100", "resident", Some("092134-shiobara"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn relation_residency_type_is_restricted() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    assert!(
        insert_relation(&pool, "092100", "commuter", None)
            .await
            .is_ok()
    );
    // v1 の値 'external' は廃止
    assert!(
        insert_relation(&pool, "092134", "external", None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn relation_is_unique_per_user_and_municipality() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    assert!(
        insert_relation(&pool, "092100", "resident", None)
            .await
            .is_ok()
    );
    assert!(
        insert_relation(&pool, "092100", "related", None)
            .await
            .is_err()
    );
    // 別の自治体とは別の関係を持てる
    assert!(
        insert_relation(&pool, "092134", "commuter", None)
            .await
            .is_ok()
    );
}

// ===== トピック =====

#[tokio::test]
async fn topic_requires_existing_municipality() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let missing = exec(
        &pool,
        "INSERT INTO topics (id, municipality_id, title, source_type, created_at) VALUES ('t9', '999999', 'x', 'citizen', '2026-10-26T00:00:00Z')",
    )
    .await;
    assert!(missing.is_err());
    let none = exec(
        &pool,
        "INSERT INTO topics (id, title, source_type, created_at) VALUES ('t9', 'x', 'citizen', '2026-10-26T00:00:00Z')",
    )
    .await;
    assert!(none.is_err());
}

// ===== ステートメント・投票 =====

#[tokio::test]
async fn statement_defaults_to_pending() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let status: String = sqlx::query_scalar("SELECT status FROM statements WHERE id = 's1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
}

#[tokio::test]
async fn statement_body_is_limited_to_140_chars() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
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
    common::seed_fixture(&pool).await;
    assert!(insert_statement(&pool, "bad", "意見", "foo").await.is_err());
}

/// Rust の状態の型（backend::shared::status）の値が、DB の CHECK と同じであること
#[tokio::test]
async fn status_enums_match_check_constraints() {
    use backend::shared::status::{Publication, StatementStatus};
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    for (i, status) in StatementStatus::ALL.into_iter().enumerate() {
        insert_statement(&pool, &format!("st{i}"), "意見", status.as_str())
            .await
            .unwrap_or_else(|e| panic!("{status:?}: {e}"));
    }
    for status in Publication::ALL {
        sqlx::query("UPDATE topics SET status = ? WHERE id = 't1'")
            .bind(status.as_str())
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("topics {status:?}: {e}"));
    }
    exec(
        &pool,
        "INSERT INTO documents (id, municipality_id, kind, title, sha256, char_count, imported_at)
         VALUES ('d', '092100', 'budget', '概要', 'h', 1, '2026-10-26T00:00:00Z')",
    )
    .await
    .unwrap();
    exec(
        &pool,
        "INSERT INTO budgets (id, municipality_id, fiscal_year, status, document_id, total, created_at)
         VALUES ('b', '092100', 2026, 'draft', 'd', 1, '2026-10-26T00:00:00Z')",
    )
    .await
    .unwrap();
    for status in Publication::ALL {
        sqlx::query("UPDATE budgets SET status = ? WHERE id = 'b'")
            .bind(status.as_str())
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("budgets {status:?}: {e}"));
    }
}

#[tokio::test]
async fn statement_requires_existing_topic() {
    let pool = common::test_pool().await;
    let res = exec(
        &pool,
        "INSERT INTO statements (id, topic_id, body, created_at) VALUES ('s', 'missing', '意見', '2026-10-26T00:00:00Z')",
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn vote_is_unique_per_user_and_statement() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    assert!(insert_vote(&pool, "v1", "s1", 1).await.is_ok());
    assert!(insert_vote(&pool, "v2", "s1", -1).await.is_err());
}

#[tokio::test]
async fn vote_value_is_restricted() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    assert!(insert_vote(&pool, "v1", "s1", 2).await.is_err());
}

// ===== タグ =====

#[tokio::test]
async fn field_tag_name_is_unique() {
    let pool = common::test_pool().await;
    assert!(
        exec(
            &pool,
            "INSERT INTO tags (id, kind, name) VALUES ('g1', 'field', '子育て')"
        )
        .await
        .is_ok()
    );
    assert!(
        exec(
            &pool,
            "INSERT INTO tags (id, kind, name) VALUES ('g2', 'field', '子育て')"
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn district_tag_requires_district_and_is_unique_per_district() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    // 地区タグには district_id が必須
    assert!(
        exec(
            &pool,
            "INSERT INTO tags (id, kind, name) VALUES ('d0', 'district', '黒羽')"
        )
        .await
        .is_err()
    );
    // 分野タグに district_id は付けられない
    assert!(
        exec(&pool, "INSERT INTO tags (id, kind, name, district_id) VALUES ('f0', 'field', '防災', '092100-kurobane')")
            .await
            .is_err()
    );
    assert!(
        exec(&pool, "INSERT INTO tags (id, kind, name, district_id) VALUES ('d1', 'district', '黒羽', '092100-kurobane')")
            .await
            .is_ok()
    );
    // 1 地区につき地区タグは 1 つ
    assert!(
        exec(&pool, "INSERT INTO tags (id, kind, name, district_id) VALUES ('d2', 'district', '黒羽地区', '092100-kurobane')")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn topic_tag_is_unique_per_topic_and_tag() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    exec(
        &pool,
        "INSERT INTO tags (id, kind, name) VALUES ('g1', 'field', '子育て')",
    )
    .await
    .unwrap();
    assert!(
        exec(
            &pool,
            "INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t1', 'g1')"
        )
        .await
        .is_ok()
    );
    assert!(
        exec(
            &pool,
            "INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t1', 'g1')"
        )
        .await
        .is_err()
    );
    // 分野タグは自治体をまたいで使える
    assert!(
        exec(
            &pool,
            "INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t2', 'g1')"
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn users_have_no_personal_attribute_columns() {
    // 本人の属性は集めない（ADR 0002）。パスキーで保存するのも公開鍵と ID と日時だけ（ADR 0006）
    let pool = common::test_pool().await;
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('users') ORDER BY cid")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        columns,
        [
            "id",
            "home_municipality_id",
            "created_at",
            "registered_at",
            "is_operator"
        ]
    );
}
