// SPDX-License-Identifier: AGPL-3.0-only

//! Claude API（Messages API）を HTTP で直接呼ぶ。Rust には公式 SDK がないため。
//! リクエストの形は Anthropic の cURL の例に合わせている（`anthropic-version` ヘッダー、
//! 構造化出力 `output_config.format`、`effort`）。

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{LlmError, Reply, Request, Usage};

pub const API_VERSION: &str = "2026-10-21";
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
/// 混雑（429・5xx）のときに再試行する回数
const MAX_RETRIES: u32 = 1;

#[derive(Clone)]
pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL.to_owned())
    }

    /// 接続先を変える（テストで手元の偽のサーバーに向けるため）
    pub fn with_base_url(api_key: String, base_url: String) -> anyhow::Result<Self> {
        // TLS の暗号処理に ring を使う（aws-lc は OpenSSL 系のライセンスを含むため使わない）。
        // 2回目以降の呼び出しでは既に設定済みなので、結果は無視する
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder().build()?;
        Ok(Self {
            http,
            api_key,
            base_url,
        })
    }

    /// `timeout` は1回の試行のタイムアウト（用途ごとに `Purpose::timeout`）
    pub async fn send(&self, body: &Value, timeout: Duration) -> Result<Reply, LlmError> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut attempt = 0;
        loop {
            let res = self
                .http
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", API_VERSION)
                .timeout(timeout)
                .json(body)
                .send()
                .await
                .map_err(|e| LlmError::Transport(e.without_url().to_string()))?;
            let status = res.status();
            if status.is_success() {
                let value: Value = res
                    .json()
                    .await
                    .map_err(|e| LlmError::Invalid(e.without_url().to_string()))?;
                return parse_reply(value);
            }
            let retryable = status.as_u16() == 429 || status.is_server_error();
            if retryable && attempt < MAX_RETRIES {
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(u64::from(attempt))).await;
                continue;
            }
            // エラーの本文は API の説明だけで、こちらが送った内容は含まない
            let message = res
                .json::<ErrorBody>()
                .await
                .map(|b| b.error.message)
                .unwrap_or_default();
            return Err(LlmError::Transport(format!("{status} {message}")));
        }
    }
}

#[derive(Deserialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
}

/// リクエストの本文を組み立てる
pub fn build_body(req: &Request) -> Value {
    json!({
        "model": req.model,
        "max_tokens": req.max_tokens,
        "system": [{
            "type": "text",
            "text": req.system,
            // 毎回同じ指示文はキャッシュして安くする（短すぎる場合はキャッシュされないだけ）
            "cache_control": {"type": "ephemeral"},
        }],
        "messages": [{"role": "user", "content": user_content(req)}],
        "output_config": {
            "effort": req.effort.as_str(),
            "format": {"type": "json_schema", "schema": req.schema},
        },
    })
}

/// 利用者の入力。PDF があれば、document ブロックとして本文の前に置く
fn user_content(req: &Request) -> Value {
    match &req.pdf {
        None => Value::String(req.user.clone()),
        Some(pdf) => json!([
            {
                "type": "document",
                "source": {
                    "type": "base64",
                    "media_type": "application/pdf",
                    "data": base64::engine::general_purpose::STANDARD.encode(pdf),
                },
            },
            {"type": "text", "text": req.user},
        ]),
    }
}

/// 応答から、止まった理由・最初のテキスト・使用量を取り出す。
/// 考える過程のブロックが先に来ることがあるので、種類が text のものを探す
pub fn parse_reply(value: Value) -> Result<Reply, LlmError> {
    #[derive(Deserialize)]
    struct Raw {
        stop_reason: Option<String>,
        #[serde(default)]
        content: Vec<Value>,
        #[serde(default)]
        usage: Usage,
    }
    let raw: Raw = serde_json::from_value(value).map_err(|e| LlmError::Invalid(e.to_string()))?;
    let text = raw
        .content
        .iter()
        .find(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .and_then(|b| b.get("text").and_then(Value::as_str))
        .map(str::to_owned);
    Ok(Reply {
        stop_reason: raw.stop_reason.unwrap_or_default(),
        text,
        usage: raw.usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{Effort, Purpose};

    #[test]
    fn body_has_structured_output_and_cache() {
        let body = build_body(&Request {
            purpose: Purpose::Moderation,
            model: "claude-sonnet-5-5".into(),
            system: "指示".into(),
            user: "本文".into(),
            schema: json!({"type": "object"}),
            effort: Effort::Low,
            max_tokens: 1024,
            pdf: None,
        });
        assert_eq!(body["model"], "claude-sonnet-5-5");
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(body["messages"][0]["content"], "本文");
        assert_eq!(body["output_config"]["effort"], "low");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    }

    #[test]
    fn pdf_goes_before_the_text() {
        let body = build_body(&Request {
            purpose: Purpose::Extraction,
            model: "claude-opus-5-5".into(),
            system: "指示".into(),
            user: "書き写してください".into(),
            schema: json!({"type": "object"}),
            effort: Effort::Medium,
            max_tokens: 1024,
            pdf: Some(b"%PDF".to_vec()),
        });
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["type"], "document");
        assert_eq!(content[0]["source"]["media_type"], "application/pdf");
        assert_eq!(content[0]["source"]["data"], "JVBERg==");
        assert_eq!(content[1]["text"], "書き写してください");
    }

    #[test]
    fn parse_skips_thinking_blocks() {
        let reply = parse_reply(json!({
            "stop_reason": "end_turn",
            "content": [
                {"type": "thinking", "thinking": ""},
                {"type": "text", "text": "{\"a\":1}"}
            ],
            "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 3}
        }))
        .unwrap();
        assert_eq!(reply.text.as_deref(), Some("{\"a\":1}"));
        assert_eq!(reply.usage.input_tokens, 10);
        assert_eq!(reply.usage.cache_read_input_tokens, 3);
        assert_eq!(reply.usage.cache_creation_input_tokens, 0);
    }
}
