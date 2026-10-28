// SPDX-License-Identifier: AGPL-3.0-only

//! LLM（Claude API）の呼び出し。モデレーションと自動タグ付けから使う。
//!
//! - 応答は構造化出力（JSON スキーマ）で受け取る
//! - 呼び出しごとに使用量と推定費用を `llm_usage` に記録し、1日の上限を超えたら呼ばない
//! - 安全上の理由で回答されなかった（`refusal`）ときや、出力が切れたときはエラーとして返す。
//!   どう扱うかは呼び出し側が決める（モデレーションでは人の確認に回す）
//! - テストでは `Llm::fake` で応答を差し替え、実際の API は呼ばない

pub mod client;
pub mod pricing;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use sqlx::SqlitePool;

use crate::shared::util::{new_id, now_iso};
use client::AnthropicClient;

/// 呼び出しの用途（`llm_usage.purpose`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Moderation,
    Tagging,
    Eval,
    /// 会議録・行政資料からの抽出。1回が高く時間もかかるので、上限とタイムアウトを別にする
    Extraction,
}

impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Moderation => "moderation",
            Self::Tagging => "tagging",
            Self::Eval => "eval",
            Self::Extraction => "extraction",
        }
    }

    /// 1回の試行のタイムアウト。抽出は長い文書を読むので長くする
    pub fn timeout(self) -> Duration {
        match self {
            Self::Extraction => Duration::from_secs(600),
            _ => Duration::from_secs(15),
        }
    }
}

/// どれだけ考えさせるか（`output_config.effort`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effort {
    Low,
    Medium,
    High,
}

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// 構造化出力の呼び出し1回分
#[derive(Debug, Clone)]
pub struct Request {
    pub purpose: Purpose,
    pub model: String,
    /// 毎回同じ指示文（プロンプトキャッシュの対象）
    pub system: String,
    /// 呼び出しごとに変わる入力
    pub user: String,
    /// 応答の JSON スキーマ
    pub schema: Value,
    pub effort: Effort,
    /// 考える過程の分も含めた出力の上限
    pub max_tokens: u32,
    /// 一緒に渡す PDF（行政資料からの抽出）。`user` の前に置く
    pub pdf: Option<Vec<u8>>,
}

/// API が返す使用量
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cache_creation_input_tokens: i64,
    #[serde(default)]
    pub cache_read_input_tokens: i64,
}

/// API の応答のうち、使う部分
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub stop_reason: String,
    /// 最初のテキストのブロック（構造化出力の JSON）
    pub text: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM の1日の費用の上限に達しました")]
    BudgetExceeded,
    #[error("安全上の理由で回答されませんでした")]
    Refusal,
    #[error("出力が上限で途中で切れました")]
    MaxTokens,
    #[error("応答を解釈できません: {0}")]
    Invalid(String),
    /// 通信の失敗・タイムアウト・API のエラー
    #[error("API の呼び出しに失敗しました: {0}")]
    Transport(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// LLM の呼び出し口。API キーがあるときだけ作る
#[derive(Clone)]
pub struct Llm {
    backend: Backend,
    /// 1日（UTC）の費用の上限（100万分の1ドル）。抽出以外
    daily_budget_micro_usd: i64,
    /// 抽出の1日の費用の上限。モデレーション・タグ付けの上限を使い切らないよう別にする
    extraction_budget_micro_usd: i64,
}

#[derive(Clone)]
enum Backend {
    Anthropic(AnthropicClient),
    /// テスト用。用意した応答を順に返し、送るはずだったリクエストを記録する
    Fake(Arc<Fake>),
}

impl Llm {
    pub fn anthropic(client: AnthropicClient, daily_budget_usd: f64) -> Self {
        Self {
            backend: Backend::Anthropic(client),
            daily_budget_micro_usd: usd_to_micro(daily_budget_usd),
            extraction_budget_micro_usd: usd_to_micro(daily_budget_usd),
        }
    }

    /// 抽出の1日の上限を設定する（既定は抽出以外と同じ額）
    pub fn with_extraction_budget(mut self, usd: f64) -> Self {
        self.extraction_budget_micro_usd = usd_to_micro(usd);
        self
    }

    /// テスト用の偽の LLM と、その応答を差し替える口
    pub fn fake(daily_budget_usd: f64) -> (Self, Arc<Fake>) {
        let fake = Arc::new(Fake::default());
        (
            Self {
                backend: Backend::Fake(fake.clone()),
                daily_budget_micro_usd: usd_to_micro(daily_budget_usd),
                extraction_budget_micro_usd: usd_to_micro(daily_budget_usd),
            },
            fake,
        )
    }

    /// 構造化出力で呼び出し、応答の JSON を返す。使用量は成否にかかわらず記録する
    pub async fn structured(&self, pool: &SqlitePool, req: &Request) -> Result<Value, LlmError> {
        let (spent, budget) = if req.purpose == Purpose::Extraction {
            (
                spent_today_extraction(pool).await?,
                self.extraction_budget_micro_usd,
            )
        } else {
            (spent_today(pool).await?, self.daily_budget_micro_usd)
        };
        if spent >= budget {
            return Err(LlmError::BudgetExceeded);
        }
        let body = client::build_body(req);
        let reply = match &self.backend {
            Backend::Anthropic(c) => c.send(&body, req.purpose.timeout()).await?,
            Backend::Fake(f) => f.send(body)?,
        };
        let result = interpret(&reply);
        let outcome = match &result {
            Ok(_) => "ok",
            Err(LlmError::Refusal) => "refusal",
            Err(LlmError::MaxTokens) => "max_tokens",
            Err(_) => "invalid",
        };
        record_usage(pool, req.purpose, &req.model, &reply.usage, outcome).await?;
        result
    }
}

/// 応答の中身を確かめて JSON を取り出す
fn interpret(reply: &Reply) -> Result<Value, LlmError> {
    match reply.stop_reason.as_str() {
        "refusal" => return Err(LlmError::Refusal),
        "max_tokens" => return Err(LlmError::MaxTokens),
        _ => {}
    }
    let text = reply
        .text
        .as_deref()
        .ok_or_else(|| LlmError::Invalid("テキストの応答がありません".into()))?;
    serde_json::from_str(text).map_err(|e| LlmError::Invalid(e.to_string()))
}

fn usd_to_micro(usd: f64) -> i64 {
    (usd * 1_000_000.0).round() as i64
}

/// 今日（UTC）の推定費用の合計（100万分の1ドル）。抽出以外
pub async fn spent_today(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    spent_since_midnight(pool, false).await
}

/// 今日（UTC）の抽出の推定費用の合計（100万分の1ドル）
pub async fn spent_today_extraction(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    spent_since_midnight(pool, true).await
}

async fn spent_since_midnight(pool: &SqlitePool, extraction: bool) -> Result<i64, sqlx::Error> {
    let start = format!("{}T00:00:00.000Z", Utc::now().format("%Y-%m-%d"));
    let (sum,): (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(cost_micro_usd) FROM llm_usage
         WHERE created_at >= $1 AND (purpose = 'extraction') = $2",
    )
    .bind(start)
    .bind(extraction)
    .fetch_one(pool)
    .await?;
    Ok(sum.unwrap_or(0))
}

async fn record_usage(
    pool: &SqlitePool,
    purpose: Purpose,
    model: &str,
    usage: &Usage,
    outcome: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO llm_usage (id, purpose, model, input_tokens, output_tokens,
             cache_creation_input_tokens, cache_read_input_tokens, cost_micro_usd, outcome, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(new_id())
    .bind(purpose.as_str())
    .bind(model)
    .bind(usage.input_tokens)
    .bind(usage.output_tokens)
    .bind(usage.cache_creation_input_tokens)
    .bind(usage.cache_read_input_tokens)
    .bind(pricing::cost_micro_usd(model, usage))
    .bind(outcome)
    .bind(now_iso())
    .execute(pool)
    .await?;
    Ok(())
}

/// テスト用の偽の LLM
#[derive(Default)]
pub struct Fake {
    replies: Mutex<VecDeque<Result<Reply, String>>>,
    requests: Mutex<Vec<Value>>,
}

impl Fake {
    /// 正常に終わった応答（構造化出力の JSON）を1件積む
    pub fn push_json(&self, value: Value) {
        self.push_reply("end_turn", Some(value.to_string()));
    }

    /// 任意の応答を1件積む
    pub fn push_reply(&self, stop_reason: &str, text: Option<String>) {
        self.replies.lock().unwrap().push_back(Ok(Reply {
            stop_reason: stop_reason.into(),
            text,
            usage: Usage {
                input_tokens: 1_000,
                output_tokens: 100,
                ..Usage::default()
            },
        }));
    }

    /// 通信の失敗を1件積む
    pub fn push_transport_error(&self, message: &str) {
        self.replies
            .lock()
            .unwrap()
            .push_back(Err(message.to_owned()));
    }

    /// これまでに送るはずだったリクエストの本文
    pub fn requests(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }

    fn send(&self, body: Value) -> Result<Reply, LlmError> {
        self.requests.lock().unwrap().push(body);
        match self.replies.lock().unwrap().pop_front() {
            Some(Ok(reply)) => Ok(reply),
            Some(Err(message)) => Err(LlmError::Transport(message)),
            None => Err(LlmError::Transport(
                "偽の LLM に応答が積まれていません".into(),
            )),
        }
    }
}
