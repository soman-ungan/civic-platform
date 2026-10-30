// SPDX-License-Identifier: AGPL-3.0-only

//! 提言（ADR 0005。`process::recommendation`・`records::recommendations`）: 締め切ったトピックだけ、
//! 合意点と意見が分かれた点をプログラムで選び、作った時点の原文と数字を残す。子の提言は項目をたどって集める。
//! 第2回のトピックは、提言の原文を最初の意見にした下書き

use backend::process::analysis::store::run_topic;
use backend::process::recommendation::{self, CreateError, Ineligible};
use backend::records::recommendations::{self, StatusError};
use backend::shared::status::{ItemKind, Publication};
use sqlx::SqlitePool;

use crate::common::seed_polarized;

const OTAWARA: &str = "092100";
const NASU: &str = "092134";
const PAST: &str = "2026-01-01T00:00:00.000Z";

/// 2つのグループに分かれた t1 を分析し、締め切る
async fn closed_topic(pool: &SqlitePool) {
    seed_polarized(pool).await;
    run_topic(pool, "t1", false).await.unwrap();
    sqlx::query("UPDATE topics SET closes_at = $1 WHERE id = 't1'")
        .bind(PAST)
        .execute(pool)
        .await
        .unwrap();
}

async fn create(
    pool: &SqlitePool,
    topics: &[&str],
    children: &[&str],
) -> Result<String, CreateError> {
    let topics: Vec<String> = topics.iter().map(|t| (*t).to_owned()).collect();
    let children: Vec<String> = children.iter().map(|t| (*t).to_owned()).collect();
    recommendation::create(
        pool,
        OTAWARA,
        &topics,
        &children,
        recommendation::DEFAULT_DIVIDED_THRESHOLD,
    )
    .await
}

#[tokio::test]
async fn only_closed_topics_can_be_used() {
    let pool = crate::common::test_pool().await;
    seed_polarized(&pool).await;
    run_topic(&pool, "t1", false).await.unwrap();
    assert!(matches!(
        create(&pool, &["t1"], &[]).await,
        Err(CreateError::Ineligible(Ineligible::NotClosed(_)))
    ));
    assert!(matches!(
        create(&pool, &[], &[]).await,
        Err(CreateError::Empty)
    ));
    // 別の自治体のトピックは見つからない
    sqlx::query("UPDATE topics SET closes_at = $1 WHERE id = 't1'")
        .bind(PAST)
        .execute(&pool)
        .await
        .unwrap();
    let topics = vec!["t1".to_owned()];
    assert!(matches!(
        recommendation::create(&pool, NASU, &topics, &[], 0.6).await,
        Err(CreateError::Ineligible(Ineligible::NotFound(_)))
    ));
}

#[tokio::test]
async fn picks_consensus_and_divided_and_keeps_the_numbers() {
    let pool = crate::common::test_pool().await;
    closed_topic(&pool).await;
    let (candidates, _) = recommendation::candidates(&pool, OTAWARA).await.unwrap();
    assert_eq!(
        candidates.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["t1"]
    );
    assert!(
        recommendation::candidates(&pool, NASU)
            .await
            .unwrap()
            .0
            .is_empty()
    );

    let id = create(&pool, &["t1"], &[]).await.unwrap();
    let detail = recommendations::show(&pool, &id).await.unwrap().unwrap();
    assert_eq!(detail.status, "draft");
    assert_eq!(detail.divided_threshold, 0.6);
    let consensus: Vec<&str> = detail
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Consensus)
        .map(|i| i.statement_id.as_str())
        .collect();
    assert_eq!(consensus, ["common"]);
    let divided = detail
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Divided)
        .count();
    assert_eq!(divided, 6);
    assert_eq!(detail.items[0].agree, 20);
    assert_eq!(detail.topics[0].participants, 20);

    // あとで意見の本文や分析が変わっても、提言の写しは変わらない
    sqlx::query("UPDATE statements SET body = '書き換え' WHERE id = 'common'")
        .execute(&pool)
        .await
        .unwrap();
    let again = recommendations::show(&pool, &id).await.unwrap().unwrap();
    assert_eq!(again.items[0].body, "全員賛成");
}

#[tokio::test]
async fn residency_is_suppressed() {
    let pool = crate::common::test_pool().await;
    closed_topic(&pool).await;
    // 住民 6人・通勤通学 2人。2人は隠し、合計が1〜4人なので次に少ない住民も隠す
    for u in 0..8 {
        let kind = if u < 6 { "resident" } else { "commuter" };
        sqlx::query(
            "INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, created_at)
             VALUES ($1, '092100', $2, $3)",
        )
        .bind(format!("v{u:02}"))
        .bind(kind)
        .bind(PAST)
        .execute(&pool)
        .await
        .unwrap();
    }
    let id = create(&pool, &["t1"], &[]).await.unwrap();
    let detail = recommendations::show(&pool, &id).await.unwrap().unwrap();
    assert!(detail.topics[0].residency.values().all(Option::is_none));
}

#[tokio::test]
async fn publish_needs_a_title_and_published_ones_do_not_change() {
    let pool = crate::common::test_pool().await;
    closed_topic(&pool).await;
    let id = create(&pool, &["t1"], &[]).await.unwrap();
    assert_eq!(
        recommendations::set_status(&pool, &id, Publication::Published)
            .await
            .unwrap(),
        Err(StatusError::NoTitle)
    );
    recommendations::edit(&pool, &id, Some("路線バスについての提言"), Some("前書き"))
        .await
        .unwrap()
        .unwrap();
    recommendations::set_status(&pool, &id, Publication::Published)
        .await
        .unwrap()
        .unwrap();
    assert!(
        recommendations::edit(&pool, &id, Some("直す"), None)
            .await
            .unwrap()
            .is_err()
    );
    assert!(!recommendations::remove_draft(&pool, &id).await.unwrap());
    let listed = recommendations::list(&pool, OTAWARA, Publication::Published)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(
        recommendations::list(&pool, NASU, Publication::Published)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn children_are_gathered_by_their_items() {
    let pool = crate::common::test_pool().await;
    closed_topic(&pool).await;
    let child = create(&pool, &["t1"], &[]).await.unwrap();
    // 下書きは子にできない
    assert!(matches!(
        create(&pool, &[], &[&child]).await,
        Err(CreateError::InvalidChild(_))
    ));
    recommendations::edit(&pool, &child, Some("子の見出し"), Some("子の前書き"))
        .await
        .unwrap()
        .unwrap();
    recommendations::set_status(&pool, &child, Publication::Published)
        .await
        .unwrap()
        .unwrap();

    // 同じトピックを直接にも選んだときは、同じ意見を重ねない
    let parent = create(&pool, &["t1"], &[&child]).await.unwrap();
    let detail = recommendations::show(&pool, &parent)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.items.len(), 7);
    assert_eq!(detail.children[0].id, child);
    // 子の見出し・前書きは使わない（親は空のまま）
    assert_eq!(detail.title, "");
    // 子から見て、親は公開されるまで出ない
    let c = recommendations::show(&pool, &child).await.unwrap().unwrap();
    assert!(c.parents.is_empty());
}

#[tokio::test]
async fn second_round_topic_starts_from_the_items() {
    let pool = crate::common::test_pool().await;
    closed_topic(&pool).await;
    let id = create(&pool, &["t1"], &[]).await.unwrap();
    assert!(
        recommendations::second_round(&pool, &id)
            .await
            .unwrap()
            .is_err()
    );
    recommendations::edit(&pool, &id, Some("見出し"), None)
        .await
        .unwrap()
        .unwrap();
    recommendations::set_status(&pool, &id, Publication::Published)
        .await
        .unwrap()
        .unwrap();
    let topic = recommendations::second_round(&pool, &id)
        .await
        .unwrap()
        .unwrap();
    let (status, source_type, rec): (String, String, Option<String>) =
        sqlx::query_as("SELECT status, source_type, recommendation_id FROM topics WHERE id = $1")
            .bind(&topic)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (status.as_str(), source_type.as_str()),
        ("draft", "citizen")
    );
    assert_eq!(rec.as_deref(), Some(id.as_str()));
    let seeds: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements WHERE topic_id = $1 AND is_seed AND user_id IS NULL",
    )
    .bind(&topic)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(seeds, 7);
}

#[tokio::test]
async fn ai_writes_only_the_heading_and_names_are_refused() {
    use backend::shared::llm::Llm;
    use serde_json::json;

    let pool = crate::common::test_pool().await;
    closed_topic(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO public_officials (id, municipality_id, name, role, term_start) VALUES
            ('o1', '092100', '大田原太郎', 'mayor', '2024-01-01')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = create(&pool, &["t1"], &[]).await.unwrap();
    let (llm, fake) = Llm::fake(1.0);

    fake.push_json(json!({"title": "大田原太郎市長への提言", "preface": "前書き"}));
    assert!(matches!(
        recommendation::draft_text(&pool, &llm, "m", &id).await,
        Err(recommendation::TextError::Name(_))
    ));
    assert_eq!(
        recommendations::show(&pool, &id)
            .await
            .unwrap()
            .unwrap()
            .title,
        ""
    );

    fake.push_json(
        json!({"title": "  全員賛成についての提言  ", "preface": "一致した点と分かれた点の紹介"}),
    );
    let (title, _) = recommendation::draft_text(&pool, &llm, "m", &id)
        .await
        .unwrap();
    assert_eq!(title, "全員賛成についての提言");
    // 送るのは元のトピックの題と項目の原文だけ（参加者・トピック・意見の ID は送らない）
    let sent = fake.requests()[1].to_string();
    assert!(sent.contains("全員賛成") && sent.contains("大田原のテスト"));
    for secret in ["v00", "\"t1\"", "common", "a0"] {
        assert!(!sent.contains(secret), "{secret}");
    }
}
