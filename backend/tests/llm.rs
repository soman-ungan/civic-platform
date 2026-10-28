// SPDX-License-Identifier: AGPL-3.0-only

//! LLM の共通基盤: 応答の解釈、使用量の記録、1日の上限、HTTP の呼び出し。
//! 実際の API は呼ばない（偽の LLM と、手元の偽のサーバーを使う）。

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use backend::shared::llm::client::{API_VERSION, AnthropicClient};
use backend::shared::llm::{Effort, Llm, LlmError, Purpose, Request, spent_today};
use serde_json::{Value, json};
use sqlx::SqlitePool;

fn request() -> Request {
    Request {
        purpose: Purpose::Moderation,
        model: "claude-sonnet-5-5".into(),
        system: "指示".into(),
        user: "本文".into(),
        schema: json!({"type": "object"}),
        effort: Effort::Low,
        max_tokens: 1024,
        pdf: None,
    }
}

async fn usage_rows(pool: &SqlitePool) -> Vec<(String, String, i64)> {
    sqlx::query_as("SELECT purpose, outcome, cost_micro_usd FROM llm_usage ORDER BY created_at")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn returns_json_and_records_usage() {
    let pool = common::test_pool().await;
    let (llm, fake) = Llm::fake(1.0);
    fake.push_json(json!({"verdict": "approved"}));

    let value = llm.structured(&pool, &request()).await.unwrap();
    assert_eq!(value["verdict"], "approved");
    // 偽の応答の使用量: 入力 1,000・出力 100 → Sonnet 5.5 で 3,000 マイクロドル
    assert_eq!(
        usage_rows(&pool).await,
        vec![("moderation".into(), "ok".into(), 3_000)]
    );
    assert_eq!(fake.requests().len(), 1);
}

#[tokio::test]
async fn refusal_and_max_tokens_are_errors_but_recorded() {
    let pool = common::test_pool().await;
    let (llm, fake) = Llm::fake(1.0);
    fake.push_reply("refusal", None);
    fake.push_reply("max_tokens", Some("{\"verdict\":".into()));
    fake.push_reply("end_turn", Some("JSON ではない".into()));

    let req = request();
    assert!(matches!(
        llm.structured(&pool, &req).await,
        Err(LlmError::Refusal)
    ));
    assert!(matches!(
        llm.structured(&pool, &req).await,
        Err(LlmError::MaxTokens)
    ));
    assert!(matches!(
        llm.structured(&pool, &req).await,
        Err(LlmError::Invalid(_))
    ));
    let outcomes: Vec<String> = usage_rows(&pool).await.into_iter().map(|r| r.1).collect();
    assert_eq!(outcomes, ["refusal", "max_tokens", "invalid"]);
}

#[tokio::test]
async fn transport_error_is_not_recorded() {
    let pool = common::test_pool().await;
    let (llm, fake) = Llm::fake(1.0);
    fake.push_transport_error("timeout");
    assert!(matches!(
        llm.structured(&pool, &request()).await,
        Err(LlmError::Transport(_))
    ));
    assert!(usage_rows(&pool).await.is_empty());
}

#[tokio::test]
async fn stops_calling_after_daily_budget() {
    let pool = common::test_pool().await;
    // 上限 0.005 ドル。1回 0.003 ドルなので、2回目までは呼び、3回目は呼ばない
    let (llm, fake) = Llm::fake(0.005);
    for _ in 0..3 {
        fake.push_json(json!({}));
    }
    let req = request();
    llm.structured(&pool, &req).await.unwrap();
    llm.structured(&pool, &req).await.unwrap();
    assert!(matches!(
        llm.structured(&pool, &req).await,
        Err(LlmError::BudgetExceeded)
    ));
    assert_eq!(fake.requests().len(), 2);
    assert_eq!(spent_today(&pool).await.unwrap(), 6_000);
}

/// 手元に偽の Messages API を立てる。最初の `fail_first` 回は 529（混雑）を返す
async fn mock_api(fail_first: usize) -> (String, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let app = axum::Router::new().route(
        "/v1/messages",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let counter = counter.clone();
            async move {
                let n = counter.fetch_add(1, Ordering::SeqCst);
                assert_eq!(headers["x-api-key"], "test-key");
                assert_eq!(headers["anthropic-version"], API_VERSION);
                assert_eq!(body["output_config"]["format"]["type"], "json_schema");
                if n < fail_first {
                    return (
                        StatusCode::from_u16(529).unwrap(),
                        Json(json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}})),
                    );
                }
                (
                    StatusCode::OK,
                    Json(json!({
                        "stop_reason": "end_turn",
                        "content": [{"type": "text", "text": "{\"ok\":true}"}],
                        "usage": {"input_tokens": 100, "output_tokens": 10}
                    })),
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), calls)
}

#[tokio::test]
async fn http_client_sends_headers_and_retries_once() {
    let pool = common::test_pool().await;
    let (url, calls) = mock_api(1).await;
    let client = AnthropicClient::with_base_url("test-key".into(), url).unwrap();
    let llm = Llm::anthropic(client, 1.0);

    let value = llm.structured(&pool, &request()).await.unwrap();
    assert_eq!(value, json!({"ok": true}));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn http_client_gives_up_after_retry() {
    let pool = common::test_pool().await;
    let (url, calls) = mock_api(usize::MAX).await;
    let client = AnthropicClient::with_base_url("test-key".into(), url).unwrap();
    let llm = Llm::anthropic(client, 1.0);

    let err = llm.structured(&pool, &request()).await.unwrap_err();
    assert!(
        matches!(&err, LlmError::Transport(m) if m.contains("529") && m.contains("Overloaded")),
        "{err:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
