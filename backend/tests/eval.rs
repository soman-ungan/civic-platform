// SPDX-License-Identifier: AGPL-3.0-only

//! 評価用データ（evals/*.jsonl）の形式と、評価の集計。実際の API は呼ばない。

mod common;

use std::collections::HashSet;
use std::path::Path;

use backend::eval::{self, ModerationCase, TaggingCase};
use backend::llm::Llm;
use backend::moderation::Moderation;
use backend::tagging::Tagger;
use serde_json::json;
use sqlx::SqlitePool;

async fn seeded_pool() -> SqlitePool {
    let pool = common::test_pool().await;
    backend::seed::run(&pool, Path::new("../data/seeds"))
        .await
        .unwrap();
    pool
}

#[test]
fn moderation_cases_are_valid() {
    let cases: Vec<ModerationCase> = eval::load(Path::new("../evals/moderation.jsonl")).unwrap();
    assert!(cases.len() >= 50, "評価データが少なすぎる: {}", cases.len());
    let mut ids = HashSet::new();
    for c in &cases {
        assert!(ids.insert(&c.id), "id の重複: {}", c.id);
        assert!(
            ["approved", "rejected", "flagged"].contains(&c.expected.as_str()),
            "{}: {}",
            c.id,
            c.expected
        );
        let chars = c.body.chars().count();
        assert!((1..=140).contains(&chars), "{}: {chars} 文字", c.id);
    }
    // どの判定のケースもある
    for expected in ["approved", "rejected", "flagged"] {
        assert!(cases.iter().any(|c| c.expected == expected));
    }
}

#[tokio::test]
async fn tagging_cases_use_vocabulary_and_own_districts() {
    let pool = seeded_pool().await;
    let cases: Vec<TaggingCase> = eval::load(Path::new("../evals/tagging.jsonl")).unwrap();
    assert!(cases.len() >= 30);
    let fields: HashSet<String> = sqlx::query_scalar("SELECT name FROM tags WHERE kind = 'field'")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .collect();
    for c in &cases {
        assert!(
            (1..=backend::tagging::MAX_FIELDS).contains(&c.fields.len()),
            "{}",
            c.id
        );
        assert!(
            c.districts.len() <= backend::tagging::MAX_DISTRICTS,
            "{}",
            c.id
        );
        for f in &c.fields {
            assert!(fields.contains(f), "{}: 語彙にない分野 {f}", c.id);
        }
        let districts: HashSet<String> =
            sqlx::query_scalar("SELECT name FROM districts WHERE municipality_id = $1")
                .bind(&c.municipality_id)
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .collect();
        for d in &c.districts {
            assert!(districts.contains(d), "{}: その自治体にない地区 {d}", c.id);
        }
    }
}

#[tokio::test]
async fn moderation_report_counts_misses() {
    let pool = seeded_pool().await;
    let (llm, fake) = Llm::fake(1.0);
    fake.push_json(json!({"verdict": "approved", "reason": "", "suggestion": ""}));
    fake.push_json(json!({"verdict": "approved", "reason": "", "suggestion": ""}));
    fake.push_reply("refusal", None);
    let cases = vec![
        case("a", "approved"),
        case("b", "rejected"),
        case("c", "flagged"),
    ];
    let moderation = Moderation::Llm {
        llm,
        model: "claude-sonnet-5-5".into(),
    };
    let report = eval::run_moderation(&pool, &moderation, &cases)
        .await
        .unwrap();
    assert_eq!(report.total, 3);
    assert_eq!(report.correct, 2);
    assert_eq!(report.misses.len(), 1);
    // 偽の応答 3 回分（3,000 マイクロドル × 3）が評価の費用として数えられる
    assert_eq!(report.cost_micro_usd, 9_000);
    let text = report.render();
    assert!(
        text.contains("見逃し（非公開にすべきものを公開）: 1"),
        "{text}"
    );

    let purposes: Vec<String> = sqlx::query_scalar("SELECT DISTINCT purpose FROM llm_usage")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(purposes, ["eval"]);
}

fn case(id: &str, expected: &str) -> ModerationCase {
    ModerationCase {
        id: id.into(),
        body: "本文".into(),
        expected: expected.into(),
        note: String::new(),
    }
}

#[tokio::test]
async fn tagging_report_compares_by_name() {
    let pool = seeded_pool().await;
    let (llm, fake) = Llm::fake(1.0);
    fake.push_json(json!({"fields": ["交通"], "districts": ["黒羽"]}));
    fake.push_json(json!({"fields": ["交通", "福祉"], "districts": []}));
    let cases = vec![
        TaggingCase {
            id: "x".into(),
            municipality_id: "092100".into(),
            title: "バス".into(),
            description: String::new(),
            fields: vec!["交通".into()],
            districts: vec!["黒羽".into()],
        },
        TaggingCase {
            id: "y".into(),
            municipality_id: "092100".into(),
            title: "バス".into(),
            description: String::new(),
            fields: vec!["交通".into()],
            districts: vec![],
        },
    ];
    let tagger = Tagger {
        llm,
        model: "claude-haiku-5-5".into(),
    };
    let report = eval::run_tagging(&pool, &tagger, &cases).await.unwrap();
    assert_eq!(report.exact, 1);
    assert_eq!(report.fields.true_positive, 2);
    assert_eq!(report.fields.false_positive, 1);
    assert_eq!(report.districts.true_positive, 1);
    // 評価は DB にタグを保存しない
    let saved: i64 = sqlx::query_scalar("SELECT count(*) FROM topic_tags")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(saved, 0);
}
