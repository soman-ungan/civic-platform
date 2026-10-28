// SPDX-License-Identifier: AGPL-3.0-only

//! 下書きのトピックは、運営者が公開するまで API のどこからも見えない。

mod common;

use axum::http::StatusCode;
use backend::drafts;
use backend::process::analysis::store::run_all;
use backend::shared::status::Publication;
use common::{request, seed_polarized, test_pool, test_state};
use serde_json::json;

const T1: &str = "/api/municipalities/otawara/topics/t1";

#[tokio::test]
async fn draft_is_hidden_until_published() {
    let pool = test_pool().await;
    // 投票と承認済みの意見がある t1 を、下書きにする
    seed_polarized(&pool).await;
    sqlx::query("UPDATE topics SET status = 'draft' WHERE id = 't1'")
        .execute(&pool)
        .await
        .unwrap();
    // 分析のワーカーは下書きを計算しない（計算するのは那須塩原市の t2 だけ）
    assert_eq!(run_all(&pool, true).await.unwrap(), 1);
    let state = test_state(pool.clone());

    let list = request(
        &state,
        "GET",
        "/api/municipalities/otawara/topics",
        None,
        None,
    )
    .await;
    assert_eq!(list.json["topics"].as_array().unwrap().len(), 0);
    for path in [
        T1.to_string(),
        format!("{T1}/next-statement"),
        format!("{T1}/analysis"),
    ] {
        let res = request(&state, "GET", &path, None, None).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{path}");
    }
    let vote = request(
        &state,
        "POST",
        &format!("{T1}/statements/a0/votes"),
        None,
        Some(json!({"value": 1})),
    )
    .await;
    assert_eq!(vote.status, StatusCode::NOT_FOUND);
    let post = request(
        &state,
        "POST",
        &format!("{T1}/statements"),
        None,
        Some(json!({"body": "意見"})),
    )
    .await;
    assert_eq!(post.status, StatusCode::NOT_FOUND);

    assert_eq!(drafts::list(&pool).await.unwrap().len(), 1);
    assert!(
        drafts::set_status(&pool, "t1", Publication::Published)
            .await
            .unwrap()
    );
    assert!(drafts::list(&pool).await.unwrap().is_empty());

    let detail = request(&state, "GET", T1, None, None).await;
    assert_eq!(detail.status, StatusCode::OK);
    assert_eq!(run_all(&pool, true).await.unwrap(), 2);

    // 保管すると、また見えなくなる
    assert!(
        drafts::set_status(&pool, "t1", Publication::Archived)
            .await
            .unwrap()
    );
    let detail = request(&state, "GET", T1, None, None).await;
    assert_eq!(detail.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn operator_can_edit_topic_statements_and_tags() {
    let pool = test_pool().await;
    seed_polarized(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO tags (id, kind, name) VALUES ('field-koutsu', 'field', '交通・移動');
         INSERT INTO tags (id, kind, name, district_id) VALUES
             ('district-092100-kurobane', 'district', '黒羽', '092100-kurobane'),
             ('district-092134-shiobara', 'district', '塩原', '092134-shiobara');
         INSERT INTO topic_tags (topic_id, tag_id, assigned_by) VALUES ('t1', 'field-koutsu', 'ai');",
    )
    .execute(&pool)
    .await
    .unwrap();

    drafts::edit_topic(&pool, "t1", None, Some(" 短い説明 "))
        .await
        .unwrap();
    let (title, description): (String, String) =
        sqlx::query_as("SELECT title, description FROM topics WHERE id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (title.as_str(), description.as_str()),
        ("大田原のテスト", "短い説明")
    );
    assert!(
        drafts::edit_topic(&pool, "missing", Some("x"), None)
            .await
            .is_err()
    );

    // 投票のある意見は直せない。投票の無い意見は直せる
    assert!(
        drafts::edit_statement(&pool, "a0", "直した意見")
            .await
            .is_err()
    );
    let sid = drafts::add_statement(&pool, "t1", "足した意見")
        .await
        .unwrap();
    drafts::edit_statement(&pool, &sid, "直した意見")
        .await
        .unwrap();
    assert!(
        drafts::add_statement(&pool, "t1", &"あ".repeat(141))
            .await
            .is_err()
    );

    // 地区タグは、そのトピックの自治体の地区だけ。AI のタグは運営者のタグになる
    drafts::set_tag(&pool, "t1", "district-092100-kurobane", true)
        .await
        .unwrap();
    assert!(
        drafts::set_tag(&pool, "t1", "district-092134-shiobara", true)
            .await
            .is_err()
    );
    drafts::set_tag(&pool, "t1", "field-koutsu", true)
        .await
        .unwrap();
    // まだタグの無い地区でも、その自治体の地区なら付けられる（タグを作る）
    sqlx::query("INSERT INTO districts (id, municipality_id, name, name_kana) VALUES ('092100-sakuyama', '092100', '佐久山', 'さくやま')")
        .execute(&pool)
        .await
        .unwrap();
    drafts::set_tag(&pool, "t1", "district-092100-sakuyama", true)
        .await
        .unwrap();
    drafts::set_tag(&pool, "t1", "district-092100-sakuyama", false)
        .await
        .unwrap();
    assert!(
        drafts::set_tag(&pool, "t1", "district-092100-missing", true)
            .await
            .is_err()
    );
    let tags: Vec<(String, String)> = sqlx::query_as(
        "SELECT tag_id, assigned_by FROM topic_tags WHERE topic_id = 't1' ORDER BY tag_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        tags,
        [
            ("district-092100-kurobane".to_string(), "admin".to_string()),
            ("field-koutsu".to_string(), "admin".to_string()),
        ]
    );
    drafts::set_tag(&pool, "t1", "field-koutsu", false)
        .await
        .unwrap();
    // 外したあと、AI が付け直さないようタグ付けは済みになる
    let tagged_at: Option<String> =
        sqlx::query_scalar("SELECT tagged_at FROM topics WHERE id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(tagged_at.is_some());
}
