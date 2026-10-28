// SPDX-License-Identifier: AGPL-3.0-only

//! 会議録の手動取り込み。架空の会議録と偽の LLM で確かめる（実際の API は呼ばない）。

mod common;

use std::sync::Arc;

use backend::ingest::extraction::Extraction;
use backend::ingest::minutes::{Input, Outcome, import, prepare};
use backend::process::moderation::Moderation;
use backend::shared::llm::{Fake, Llm};
use common::{seed_fixture, test_pool};
use serde_json::{Value, json};
use sqlx::SqlitePool;

/// 架空の会議録（名前はすべて架空）
const MINUTES: &str = "\
令和7年第3回大田原市議会定例会会議録（架空）
日程第　１　一般質問

○議長（高橋　一郎君）　ただいまから本日の会議を開きます。
◆12番（鈴木　花子君）　通告に従い質問します。
放課後児童クラブの待機児童が増えています。市の対策を伺います。
◎市長（山田一郎君）　来年度に定員を40人増やす方針です。
◎保健福祉部長（佐藤次郎君）　民間の事業者とも協議しています。
";

fn input() -> Input {
    Input {
        municipality_id: "092100".into(),
        title: "令和7年第3回定例会（架空）".into(),
        held_on: "2025-09-05".into(),
        url: Some("https://example.invalid/minutes/1".into()),
        text: MINUTES.into(),
    }
}

async fn setup(with_roster: bool) -> (SqlitePool, Llm, Arc<Fake>, Moderation) {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    if with_roster {
        sqlx::raw_sql(
            "INSERT INTO public_officials (id, municipality_id, name, role, term_start) VALUES
                 ('o1', '092100', '鈴木 花子', 'councilor', '2023-05-01'),
                 ('o2', '092100', '高橋一郎', 'councilor', '2023-05-01'),
                 ('o3', '092100', '山田一郎', 'mayor', '2024-01-01'),
                 -- 別の自治体の同名の議員とは対応付けない
                 ('o9', '092134', '鈴木花子', 'councilor', '2023-05-01');",
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let (llm, fake) = Llm::fake(1.0);
    let moderation = Moderation::Llm {
        llm: llm.clone(),
        model: "claude-sonnet-5-5".into(),
    };
    (pool, llm, fake, moderation)
}

fn extraction_reply() -> Value {
    json!({"topics": [
        {
            "title": "放課後児童クラブの待機児童をどう減らすか",
            "description": "議員から待機児童の対策について質問があり、市は定員を増やす方針を答えた。",
            "sources": [
                {"utterance_no": 2, "quote": "放課後児童クラブの待機児童が増えています。"},
                {"utterance_no": 3, "quote": "来年度に定員を40人増やす方針です。"},
                {"utterance_no": 4, "quote": "民間の事業者とも協議しています。"},
                {"utterance_no": 3, "quote": "定員を100人増やします。"}
            ],
            "statements": [
                {"body": "市の施設の定員を増やすべきだ", "basis": [3]},
                {"body": "民間の学童への補助を増やすべきだ", "basis": [4]},
                {"body": "待機児童の対策を急ぐべきだ", "basis": [2, 3]},
                {"body": "鈴木花子議員の提案に賛成だ", "basis": [2]},
                {"body": "会議録に根拠の無い新しい案", "basis": []}
            ]
        },
        {
            "title": "引用が作られたトピック",
            "description": "説明",
            "sources": [{"utterance_no": 2, "quote": "会議録に無い文"}],
            "statements": [
                {"body": "a", "basis": [2]},
                {"body": "b", "basis": [2]},
                {"body": "c", "basis": [2]}
            ]
        }
    ]})
}

fn approve() -> Value {
    json!({"verdict": "approved", "reason": "", "suggestion": ""})
}

#[tokio::test]
async fn stops_when_speakers_are_not_in_roster() {
    let (pool, llm, fake, moderation) = setup(false).await;
    let outcome = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap();
    match outcome {
        Outcome::Unmatched(names) => assert_eq!(
            names,
            ["12番（鈴木花子）", "市長（山田一郎）", "議長（高橋一郎）"]
        ),
        other => panic!("{other:?}"),
    }
    // 職員は名簿の対象外
    assert!(fake.requests().is_empty(), "AI を呼んではいけない");
}

#[tokio::test]
async fn prepare_matches_officials_in_term_and_municipality() {
    let (pool, ..) = setup(true).await;
    let p = prepare(&pool, &input()).await.unwrap();
    assert!(p.unmatched.is_empty(), "{:?}", p.unmatched);
    assert_eq!(p.officials.get(&2).map(String::as_str), Some("o1"));
    // 任期の前の日付なら、市長は対応付けられない
    let mut early = input();
    early.held_on = "2023-12-01".into();
    let p = prepare(&pool, &early).await.unwrap();
    assert_eq!(p.unmatched, ["市長（山田一郎）"]);
    // 職員の氏名も、出してはいけない名前に入る
    assert!(p.names.contains(&"佐藤次郎".to_string()));
}

#[tokio::test]
async fn imports_checked_topics_as_drafts() {
    let (pool, llm, fake, moderation) = setup(true).await;
    fake.push_json(extraction_reply());
    // 確かめたあとに残る意見3件をモデレーション。1件目は非公開にする
    fake.push_json(json!({"verdict": "rejected", "reason": "", "suggestion": ""}));
    fake.push_json(approve());
    fake.push_json(approve());

    let Outcome::Imported(report) = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap() else {
        panic!("取り込めていない")
    };
    // モデレーションで1件減り、2件になったので捨てられる
    assert!(report.topics.is_empty(), "{:?}", report.notes);
    assert!(report.notes.iter().any(|n| n.contains("無い引用")));
    assert!(report.notes.iter().any(|n| n.contains("氏名")));
    assert!(report.notes.iter().any(|n| n.contains("根拠の発言が無い")));

    // AI に送った会議録には職員の氏名が無い
    let sent = fake.requests()[0].to_string();
    assert!(sent.contains("保健福祉部長"));
    assert!(!sent.contains("佐藤"));
    let purpose: String =
        sqlx::query_scalar("SELECT purpose FROM llm_usage ORDER BY created_at LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(purpose, "extraction");
}

#[tokio::test]
async fn saved_draft_has_sources_and_statements() {
    let (pool, llm, fake, moderation) = setup(true).await;
    fake.push_json(extraction_reply());
    for _ in 0..3 {
        fake.push_json(approve());
    }
    let Outcome::Imported(report) = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap() else {
        panic!("取り込めていない")
    };
    assert_eq!(report.topics.len(), 1, "{:?}", report.notes);
    let topic_id = &report.topics[0].0;

    let (status, source_type): (String, String) =
        sqlx::query_as("SELECT status, source_type FROM topics WHERE id = $1")
            .bind(topic_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (status.as_str(), source_type.as_str()),
        ("draft", "assembly")
    );

    let statements: Vec<(Option<String>, String)> =
        sqlx::query_as("SELECT user_id, status FROM statements WHERE topic_id = $1")
            .bind(topic_id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(statements.len(), 3);
    // 意見の根拠の発言（運営者の確認用）
    let basis: Vec<(i64, String)> = sqlx::query_as(
        "SELECT ss.utterance_no, ss.speaker_label FROM statement_sources ss
         JOIN statements s ON s.id = ss.statement_id
         WHERE s.topic_id = $1 ORDER BY ss.utterance_no",
    )
    .bind(topic_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        basis,
        [
            (2, "鈴木花子議員".to_string()),
            (3, "山田一郎市長".to_string()),
            (3, "山田一郎市長".to_string()),
            (4, "保健福祉部長".to_string()),
        ]
    );
    assert!(
        statements
            .iter()
            .all(|(u, s)| u.is_none() && s == "approved")
    );

    let sources: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT utterance_no, speaker_label, official_id FROM topic_sources
         WHERE topic_id = $1 ORDER BY position",
    )
    .bind(topic_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        sources,
        [
            (2, "鈴木花子議員".to_string(), Some("o1".to_string())),
            (3, "山田一郎市長".to_string(), Some("o3".to_string())),
            (4, "保健福祉部長".to_string(), None),
        ]
    );

    // 同じ会議録は二度取り込まない。again なら同じ文書から作り直せる
    match import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap()
    {
        Outcome::Duplicate(id) => assert_eq!(id, report.document_id),
        other => panic!("{other:?}"),
    }
    fake.push_json(json!({"topics": []}));
    let Outcome::Imported(again) = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        true,
        false,
    )
    .await
    .unwrap() else {
        panic!("作り直せていない")
    };
    assert_eq!(again.document_id, report.document_id);
    let documents: i64 = sqlx::query_scalar("SELECT count(*) FROM documents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(documents, 1);
}

#[tokio::test]
async fn text_without_speakers_is_reported() {
    let (pool, llm, _fake, moderation) = setup(true).await;
    let mut i = input();
    i.text = "見出しの形が違う会議録".into();
    assert!(matches!(
        import(
            &pool,
            &Extraction::Api {
                llm: &llm,
                model: "claude-opus-5-5",
            },
            &moderation,
            &i,
            false,
            false
        )
        .await
        .unwrap(),
        Outcome::NoUtterances
    ));
}

#[tokio::test]
async fn published_topic_shows_sources() {
    let (pool, llm, fake, moderation) = setup(true).await;
    fake.push_json(extraction_reply());
    for _ in 0..3 {
        fake.push_json(approve());
    }
    let Outcome::Imported(report) = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap() else {
        panic!("取り込めていない")
    };
    let topic_id = &report.topics[0].0;
    backend::drafts::set_status(
        &pool,
        topic_id,
        backend::shared::status::Publication::Published,
    )
    .await
    .unwrap();

    let state = common::test_state(pool);
    let res = common::request(
        &state,
        "GET",
        &format!("/api/municipalities/otawara/topics/{topic_id}"),
        None,
        None,
    )
    .await;
    let sources = res.json["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0]["speaker"], "鈴木花子議員");
    assert_eq!(sources[2]["speaker"], "保健福祉部長");
    assert_eq!(sources[0]["held_on"], "2025-09-05");
    assert_eq!(res.json["source_type"], "assembly");
    common::assert_no_key(&res.json, "official_id");
    common::assert_no_key(&res.json, "basis");
    assert!(!res.json.to_string().contains("佐藤"));
}

#[tokio::test]
async fn statement_with_failed_moderation_is_kept_as_pending() {
    let (pool, llm, fake, moderation) = setup(true).await;
    fake.push_json(extraction_reply());
    fake.push_transport_error("timeout");
    fake.push_json(approve());
    fake.push_json(approve());
    let Outcome::Imported(report) = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap() else {
        panic!("取り込めていない")
    };
    assert_eq!(report.topics.len(), 1, "{:?}", report.notes);
    let statuses: Vec<String> =
        sqlx::query_scalar("SELECT status FROM statements WHERE topic_id = $1 ORDER BY status")
            .bind(&report.topics[0].0)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(statuses, ["approved", "approved", "pending"]);
}

#[tokio::test]
async fn procedural_minutes_are_skipped_unless_forced() {
    let (pool, llm, fake, moderation) = setup(true).await;
    let mut i = input();
    i.text = i
        .text
        .replace("日程第　１　一般質問", "日程第　１　会期の決定");
    match import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &i,
        false,
        false,
    )
    .await
    .unwrap()
    {
        Outcome::NotTopical(agenda) => assert_eq!(agenda, ["日程第　１　会期の決定"]),
        other => panic!("{other:?}"),
    }
    assert!(fake.requests().is_empty(), "AI を呼んではいけない");

    fake.push_json(json!({"topics": []}));
    assert!(matches!(
        import(
            &pool,
            &Extraction::Api {
                llm: &llm,
                model: "claude-opus-5-5",
            },
            &moderation,
            &i,
            false,
            true
        )
        .await
        .unwrap(),
        Outcome::Imported(_)
    ));
}

#[tokio::test]
async fn short_chair_remarks_are_not_sent() {
    let (pool, llm, fake, moderation) = setup(true).await;
    let mut i = input();
    i.text.push_str("○議長（高橋　一郎君）　山田一郎市長。\n");
    fake.push_json(json!({"topics": []}));
    import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &i,
        false,
        false,
    )
    .await
    .unwrap();
    let sent = fake.requests()[0]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!sent.contains("高橋一郎議長"), "{sent}");
    // 発言の番号は元のまま
    assert!(sent.contains("[2] 鈴木花子議員"), "{sent}");
}

#[tokio::test]
async fn empty_topics_are_retried_once() {
    let (pool, llm, fake, moderation) = setup(true).await;
    let empty =
        json!({"topics": [{"title": "", "description": "", "sources": [], "statements": []}]});
    fake.push_json(empty.clone());
    fake.push_json(extraction_reply());
    for _ in 0..3 {
        fake.push_json(approve());
    }
    let Outcome::Imported(report) = import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        false,
        false,
    )
    .await
    .unwrap() else {
        panic!("取り込めていない")
    };
    assert_eq!(report.topics.len(), 1, "{:?}", report.notes);

    // 2回続けて空なら、やり直しは1回まで
    fake.push_json(empty.clone());
    fake.push_json(empty);
    let before = fake.requests().len();
    import(
        &pool,
        &Extraction::Api {
            llm: &llm,
            model: "claude-opus-5-5",
        },
        &moderation,
        &input(),
        true,
        false,
    )
    .await
    .unwrap();
    assert_eq!(fake.requests().len() - before, 2);
}

/// 共同作業で作った出力（manual）は API を呼ばずに、API の時と同じ検査を通って下書きになる
#[tokio::test]
async fn manual_extraction_uses_the_same_checks_without_api() {
    let (pool, _llm, fake, _) = setup(true).await;
    let manual = Extraction::Manual(extraction_reply());

    // 保存しない確かめ（preview）
    let (topics, notes) = backend::ingest::minutes::preview(&pool, &manual, &input())
        .await
        .unwrap();
    assert_eq!(topics.len(), 1);
    assert_eq!(topics[0].statements.len(), 3);
    assert!(notes.iter().any(|n| n.contains("無い引用")));

    // 取り込み（最初の意見のモデレーションは、API キーが無ければ全件承認）
    let Outcome::Imported(report) = import(
        &pool,
        &manual,
        &Moderation::AutoApprove,
        &input(),
        false,
        false,
    )
    .await
    .unwrap() else {
        panic!("取り込めていない")
    };
    assert_eq!(report.topics.len(), 1);
    assert!(fake.requests().is_empty(), "API を呼んでいる");
    let method: Option<String> =
        sqlx::query_scalar("SELECT extraction FROM documents WHERE id = $1")
            .bind(&report.document_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(method.as_deref(), Some("manual"));

    // 形の合わない JSON は、分かる言葉で止める
    let broken = Extraction::Manual(json!({"topic": []}));
    let err = backend::ingest::minutes::preview(&pool, &broken, &input())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("出力の形が合いません"), "{err}");
}
