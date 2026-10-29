// SPDX-License-Identifier: AGPL-3.0-only

//! LLM によるモデレーション。偽の LLM で各経路を確かめる（実際の API は呼ばない）。

mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use backend::interface::api::AppState;
use backend::process::moderation::{self, Moderation};
use backend::shared::llm::{Fake, Llm};
use common::{request, seed_fixture, test_pool, test_state};
use serde_json::json;
use sqlx::SqlitePool;

const T1: &str = "/api/municipalities/otawara/topics/t1";

async fn setup() -> (AppState, Arc<Fake>) {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    // fixture の s1 は pending のままなので、確認待ちに混ざらないよう承認済みにしておく
    sqlx::query("UPDATE statements SET status = 'approved' WHERE id = 's1'")
        .execute(&pool)
        .await
        .unwrap();
    let (llm, fake) = Llm::fake(1.0);
    let state = test_state(pool).with_moderation(Moderation::Llm {
        llm,
        model: "claude-sonnet-5-5".into(),
    });
    (state, fake)
}

/// パスキーを登録済みの参加者として投稿する（投稿にはパスキーの登録が要る。ADR 0006）
async fn post(state: &AppState, body: &str) -> common::TestResponse {
    let cookie = common::account_ready_to_post(&state.pool, state, "t1").await;
    request(
        state,
        "POST",
        &format!("{T1}/statements"),
        Some(&cookie),
        Some(json!({"body": body})),
    )
    .await
}

async fn stored(pool: &SqlitePool, id: &str) -> (String, Option<String>, String) {
    sqlx::query_as("SELECT status, moderation_note, user_id FROM statements WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn approved_is_published() {
    let (state, fake) = setup().await;
    fake.push_json(json!({"harm": "ok", "person": "ok", "votable": "ok", "duplicate_of": 0, "reason": "問題なし", "suggestion": ""}));

    let res = post(&state, "バスの本数を増やしてほしい").await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.json["status"], "approved");
    assert_eq!(res.json["suggestion"], serde_json::Value::Null);
    let (status, note, _) = stored(&state.pool, res.json["id"].as_str().unwrap()).await;
    assert_eq!(status, "approved");
    assert_eq!(note.as_deref(), Some("問題なし"));
    // 判定の理由は運営者向けで、API では返さない
    common::assert_no_key(&res.json, "moderation_note");
    common::assert_no_key(&res.json, "reason");
}

#[tokio::test]
async fn rejected_returns_suggestion() {
    let (state, fake) = setup().await;
    fake.push_json(json!({
        "harm": "reject",
        "person": "ok",
        "votable": "ok",
        "duplicate_of": 0,
        "reason": "人格攻撃",
        "suggestion": "  この計画には反対です。費用の根拠を示してほしい  "
    }));

    let res = post(&state, "（人格攻撃を含む投稿）").await;
    assert_eq!(res.json["status"], "rejected");
    assert_eq!(
        res.json["suggestion"],
        "この計画には反対です。費用の根拠を示してほしい"
    );
}

#[tokio::test]
async fn long_suggestion_is_cut_to_140_chars() {
    let (state, fake) = setup().await;
    fake.push_json(json!({"harm": "reject", "person": "ok", "votable": "ok", "duplicate_of": 0, "reason": "x", "suggestion": "あ".repeat(200)}));
    let res = post(&state, "投稿").await;
    assert_eq!(
        res.json["suggestion"].as_str().unwrap().chars().count(),
        140
    );
}

#[tokio::test]
async fn flagged_and_refusal_wait_for_operator() {
    let (state, fake) = setup().await;
    fake.push_json(json!({"harm": "unsure", "person": "ok", "votable": "ok", "duplicate_of": 0, "reason": "判断に迷う", "suggestion": ""}));
    fake.push_reply("refusal", None);

    let a = post(&state, "皮肉っぽい投稿").await;
    let b = post(&state, "AI が答えない投稿").await;
    assert_eq!(a.json["status"], "flagged");
    assert_eq!(b.json["status"], "flagged");

    let queue = backend::records::statements::queue(&state.pool)
        .await
        .unwrap();
    assert_eq!(queue.len(), 2);
    assert!(
        queue[1]
            .moderation_note
            .as_deref()
            .unwrap()
            .contains("安全上")
    );
}

#[tokio::test]
async fn failure_stays_pending_then_retry_decides() {
    let (state, fake) = setup().await;
    fake.push_transport_error("timeout");

    let res = post(&state, "通信に失敗する投稿").await;
    assert_eq!(res.status, StatusCode::CREATED);
    // 自動承認には戻さない
    assert_eq!(res.json["status"], "pending");
    let id = res.json["id"].as_str().unwrap();

    // 投稿の直後は再判定しない（判定中の投稿と重ならないように）
    fake.push_json(json!({"harm": "ok", "person": "ok", "votable": "ok", "duplicate_of": 0, "reason": "問題なし", "suggestion": ""}));
    assert_eq!(
        moderation::retry_pending(&state.pool, &state.moderation)
            .await
            .unwrap(),
        0
    );

    sqlx::query("UPDATE statements SET created_at = '2026-01-01T00:00:00.000Z' WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        moderation::retry_pending(&state.pool, &state.moderation)
            .await
            .unwrap(),
        1
    );
    assert_eq!(stored(&state.pool, id).await.0, "approved");
}

#[tokio::test]
async fn budget_exceeded_stays_pending() {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    // 上限 0 ドル: 呼ばずに pending
    let (llm, fake) = Llm::fake(0.0);
    let state = test_state(pool).with_moderation(Moderation::Llm {
        llm,
        model: "claude-sonnet-5-5".into(),
    });
    let res = post(&state, "上限を超えた投稿").await;
    assert_eq!(res.json["status"], "pending");
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn only_body_is_sent_to_llm() {
    let (state, fake) = setup().await;
    fake.push_json(common::approved_reply());
    let res = post(&state, "送る本文").await;
    let (_, _, user_id) = stored(&state.pool, res.json["id"].as_str().unwrap()).await;

    let sent = fake.requests();
    assert_eq!(sent.len(), 1);
    let text = sent[0].to_string();
    assert!(text.contains("送る本文"));
    assert!(!text.contains(&user_id), "参加者 ID が送られている");
    assert!(!text.contains("t1"), "トピックの ID が送られている");
}

#[tokio::test]
async fn auto_approve_does_not_retry_pending() {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    sqlx::query("UPDATE statements SET status = 'pending', created_at = '2026-01-01T00:00:00.000Z' WHERE id = 's1'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        moderation::retry_pending(&pool, &Moderation::AutoApprove)
            .await
            .unwrap(),
        0
    );
    assert_eq!(stored(&pool, "s1").await.0, "pending");
}

#[tokio::test]
async fn operator_can_approve_and_reject() {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    sqlx::query("UPDATE statements SET status = 'flagged' WHERE id = 's1'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(moderation::decide(&pool, "s1", true).await.unwrap());
    assert_eq!(stored(&pool, "s1").await.0, "approved");
    assert!(moderation::decide(&pool, "s1", false).await.unwrap());
    assert_eq!(stored(&pool, "s1").await.0, "rejected");
    assert!(!moderation::decide(&pool, "missing", true).await.unwrap());
}
