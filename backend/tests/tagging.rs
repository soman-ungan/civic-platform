// SPDX-License-Identifier: AGPL-3.0-only

//! トピックの自動タグ付け。偽の LLM で確かめる（実際の API は呼ばない）。

mod common;

use std::sync::Arc;

use backend::llm::{Fake, Llm};
use backend::tagging::{Outcome, Tagger, run_untagged, tag_topic};
use common::{seed_fixture, test_pool};
use serde_json::json;
use sqlx::SqlitePool;

async fn setup() -> (SqlitePool, Tagger, Arc<Fake>) {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO tags (id, kind, name) VALUES
             ('field-koutsu', 'field', '交通'),
             ('field-fukushi', 'field', '福祉'),
             ('field-zaisei', 'field', '財政');
         UPDATE topics SET description = '黒羽地区のバスについて。山田一郎議員が議会で取り上げた' WHERE id = 't1';",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (llm, fake) = Llm::fake(1.0);
    let tagger = Tagger {
        llm,
        model: "claude-haiku-5-5".into(),
    };
    (pool, tagger, fake)
}

async fn tags_of(pool: &SqlitePool, topic: &str) -> Vec<(String, String)> {
    sqlx::query_as("SELECT tag_id, assigned_by FROM topic_tags WHERE topic_id = $1 ORDER BY tag_id")
        .bind(topic)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn tagged_at(pool: &SqlitePool, topic: &str) -> Option<String> {
    sqlx::query_scalar("SELECT tagged_at FROM topics WHERE id = $1")
        .bind(topic)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn candidates_are_vocabulary_and_own_districts_only() {
    let (pool, tagger, fake) = setup().await;
    fake.push_json(json!({"fields": ["交通"], "districts": ["黒羽"]}));
    tag_topic(&pool, &tagger, "t1", false).await.unwrap();

    let sent = &fake.requests()[0];
    let schema = &sent["output_config"]["format"]["schema"]["properties"];
    assert_eq!(
        schema["fields"]["items"]["enum"],
        json!(["交通", "福祉", "財政"])
    );
    // 那須塩原市の地区（塩原）は候補に入らない
    assert_eq!(schema["districts"]["items"]["enum"], json!(["黒羽"]));
    // 本文に議員名があっても、候補（enum）には人物が入らない
    assert!(!schema.to_string().contains("山田"));
}

#[tokio::test]
async fn only_known_tags_are_saved_and_district_tag_is_created() {
    let (pool, tagger, fake) = setup().await;
    // 構造化出力で守られるはずだが、候補外の値（人物名・他の自治体の地区）が来ても保存しない
    fake.push_json(json!({
        "fields": ["交通", "山田一郎", "福祉", "財政", "交通"],
        "districts": ["黒羽", "塩原"]
    }));
    let outcome = tag_topic(&pool, &tagger, "t1", false).await.unwrap();
    assert_eq!(
        outcome,
        Outcome::Tagged(vec![
            "field-koutsu".into(),
            "field-fukushi".into(),
            "field-zaisei".into(),
            "district-092100-kurobane".into(),
        ])
    );
    let district: (String, String) =
        sqlx::query_as("SELECT name, district_id FROM tags WHERE id = 'district-092100-kurobane'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(district, ("黒羽".into(), "092100-kurobane".into()));
    assert!(tagged_at(&pool, "t1").await.is_some());
    assert!(tags_of(&pool, "t1").await.iter().all(|(_, by)| by == "ai"));
}

#[tokio::test]
async fn force_retags_but_keeps_admin_tags() {
    let (pool, tagger, fake) = setup().await;
    sqlx::query("INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t1', 'field-zaisei')")
        .execute(&pool)
        .await
        .unwrap();
    fake.push_json(json!({"fields": ["交通", "財政"], "districts": []}));
    tag_topic(&pool, &tagger, "t1", false).await.unwrap();
    assert_eq!(
        tags_of(&pool, "t1").await,
        [
            ("field-koutsu".to_string(), "ai".to_string()),
            ("field-zaisei".to_string(), "admin".to_string()),
        ]
    );

    // タグ付け済みなら、force でなければ呼ばない
    assert_eq!(
        tag_topic(&pool, &tagger, "t1", false).await.unwrap(),
        Outcome::Skipped
    );
    assert_eq!(fake.requests().len(), 1);

    // force: AI のタグだけを付け直す
    fake.push_json(json!({"fields": ["福祉"], "districts": []}));
    tag_topic(&pool, &tagger, "t1", true).await.unwrap();
    assert_eq!(
        tags_of(&pool, "t1").await,
        [
            ("field-fukushi".to_string(), "ai".to_string()),
            ("field-zaisei".to_string(), "admin".to_string()),
        ]
    );
}

#[tokio::test]
async fn run_untagged_skips_topics_with_tags() {
    let (pool, tagger, fake) = setup().await;
    // t2 は運営者がタグ付け済み
    sqlx::query("INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t2', 'field-zaisei')")
        .execute(&pool)
        .await
        .unwrap();
    fake.push_json(json!({"fields": [], "districts": []}));
    assert_eq!(run_untagged(&pool, &tagger).await.unwrap(), 1);
    // 当てはまるタグが無くても済みにして、繰り返し呼ばない
    assert!(tagged_at(&pool, "t1").await.is_some());
    assert_eq!(run_untagged(&pool, &tagger).await.unwrap(), 0);
    assert_eq!(fake.requests().len(), 1);
}

#[tokio::test]
async fn refusal_marks_done_without_tags() {
    let (pool, tagger, fake) = setup().await;
    fake.push_reply("refusal", None);
    tag_topic(&pool, &tagger, "t1", false).await.unwrap();
    assert!(tags_of(&pool, "t1").await.is_empty());
    assert!(tagged_at(&pool, "t1").await.is_some());
}

#[tokio::test]
async fn failure_leaves_topic_for_next_time() {
    let (pool, tagger, fake) = setup().await;
    fake.push_transport_error("timeout");
    assert!(tag_topic(&pool, &tagger, "t1", false).await.is_err());
    assert!(tagged_at(&pool, "t1").await.is_none());
}
