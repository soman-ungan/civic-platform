// SPDX-License-Identifier: AGPL-3.0-only

use std::env;

const DEFAULT_DATABASE_URL: &str = "sqlite://../data/civic.db?mode=rwc";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_SEEDS_DIR: &str = "../data/seeds";
const DEFAULT_DEV_SEEDS_DIR: &str = "../data/dev-seeds";
const DEFAULT_PRIVATE_DIR: &str = "../data/private";
const DEFAULT_WEBAUTHN_RP_ID: &str = "localhost";
const DEFAULT_WEBAUTHN_ORIGIN: &str = "http://localhost:3000";
const DEFAULT_MODERATION_MODEL: &str = "claude-sonnet-5-5";
const DEFAULT_TAGGING_MODEL: &str = "claude-sonnet-5-5";
const DEFAULT_LLM_DAILY_BUDGET_USD: f64 = 1.0;
const DEFAULT_EXTRACTION_MODEL: &str = "claude-opus-5-5";
const DEFAULT_LLM_EXTRACTION_DAILY_BUDGET_USD: f64 = 5.0;

pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub seeds_dir: String,
    /// 開発用サンプルデータ（`seed --dev` のときだけ使う）
    pub dev_seeds_dir: String,
    /// 原本の置き場（git の管理外）。`fetch` は `<ここ>/fetched/` に置く
    pub private_dir: String,
    pub session_secret: String,
    pub cookie_secure: bool,
    /// 分析の再計算の間隔（秒）。0 なら定期実行しない
    pub analysis_interval_secs: u64,
    /// Claude API のキー。無ければ AI の機能を使わない（モデレーションは自動承認、タグ付けはしない）
    pub anthropic_api_key: Option<String>,
    pub moderation_model: String,
    pub tagging_model: String,
    /// LLM の1日（UTC）の費用の上限（米ドル）。超えたら呼ばない
    pub llm_daily_budget_usd: f64,
    /// 会議録などからの抽出に使うモデル
    pub extraction_model: String,
    /// 抽出の1日（UTC）の費用の上限（米ドル）。モデレーション・タグ付けとは別枠
    pub llm_extraction_daily_budget_usd: f64,
    /// 収集の User-Agent に入れる運営者の連絡先（URL かメールアドレス）。無ければ収集しない
    pub collector_contact: Option<String>,
    /// 提言の「意見が分かれた点」のしきい値（あるグループで賛成・別のグループで反対が、どちらもこれ以上）。
    /// 作った提言に使った値を残す（process::recommendation）
    pub recommendation_divided_threshold: f64,
    /// パスキーを結び付けるドメイン（ADR 0006）。本番ではサービスのドメイン
    pub webauthn_rp_id: String,
    /// パスキーの登録・ログインを行う画面のオリジン（ブラウザから見た URL）
    pub webauthn_origin: String,
}

impl Config {
    /// 環境変数から読み込む。未設定の項目は開発用のデフォルト値を使う。
    /// `SESSION_SECRET` だけは既定値を持たず、未設定ならエラーにする。
    pub fn from_env() -> anyhow::Result<Self> {
        let session_secret = env::var("SESSION_SECRET").map_err(|_| {
            anyhow::anyhow!("SESSION_SECRET が設定されていません（.env.example を参照）")
        })?;
        Ok(Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into()),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into()),
            seeds_dir: env::var("SEEDS_DIR").unwrap_or_else(|_| DEFAULT_SEEDS_DIR.into()),
            dev_seeds_dir: env::var("DEV_SEEDS_DIR")
                .unwrap_or_else(|_| DEFAULT_DEV_SEEDS_DIR.into()),
            private_dir: env::var("PRIVATE_DIR").unwrap_or_else(|_| DEFAULT_PRIVATE_DIR.into()),
            session_secret,
            cookie_secure: env::var("COOKIE_SECURE").is_ok_and(|v| v == "true"),
            analysis_interval_secs: env::var("ANALYSIS_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            moderation_model: env::var("MODERATION_MODEL")
                .unwrap_or_else(|_| DEFAULT_MODERATION_MODEL.into()),
            tagging_model: env::var("TAGGING_MODEL")
                .unwrap_or_else(|_| DEFAULT_TAGGING_MODEL.into()),
            llm_daily_budget_usd: env::var("LLM_DAILY_BUDGET_USD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LLM_DAILY_BUDGET_USD),
            extraction_model: env::var("EXTRACTION_MODEL")
                .unwrap_or_else(|_| DEFAULT_EXTRACTION_MODEL.into()),
            llm_extraction_daily_budget_usd: env::var("LLM_EXTRACTION_DAILY_BUDGET_USD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LLM_EXTRACTION_DAILY_BUDGET_USD),
            collector_contact: env::var("COLLECTOR_CONTACT")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            recommendation_divided_threshold: env::var("RECOMMENDATION_DIVIDED_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|v: &f64| (0.5..1.0).contains(v))
                .unwrap_or(crate::process::recommendation::DEFAULT_DIVIDED_THRESHOLD),
            webauthn_rp_id: env::var("WEBAUTHN_RP_ID")
                .unwrap_or_else(|_| DEFAULT_WEBAUTHN_RP_ID.into()),
            webauthn_origin: env::var("WEBAUTHN_ORIGIN")
                .unwrap_or_else(|_| DEFAULT_WEBAUTHN_ORIGIN.into()),
        })
    }
}
