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

#[derive(Debug, Clone, Default)]
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
    /// パスキーを結び付けるドメイン（ADR 0006）。本番ではサービスのドメイン
    pub webauthn_rp_id: String,
    /// パスキーの登録・ログインを行う画面のオリジン（ブラウザから見た URL）
    pub webauthn_origin: String,
    /// 接続元の IP を `X-Forwarded-For` から読むか（本番の Caddy の後ろで true。回数の制限に使う）
    pub trust_forwarded_for: bool,
}

impl Config {
    /// 環境変数から読み込む。未設定の項目は開発用のデフォルト値を使う。
    /// `SESSION_SECRET` だけは既定値を持たず、未設定ならエラーにする。
    /// 空の値は未設定と同じに扱う（docker compose は未設定の変数を空で渡すため）
    pub fn from_env() -> anyhow::Result<Self> {
        let session_secret = var("SESSION_SECRET").map_err(|_| {
            anyhow::anyhow!("SESSION_SECRET が設定されていません（.env.example を参照）")
        })?;
        Ok(Self {
            database_url: var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into()),
            bind_addr: var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into()),
            seeds_dir: var("SEEDS_DIR").unwrap_or_else(|_| DEFAULT_SEEDS_DIR.into()),
            dev_seeds_dir: var("DEV_SEEDS_DIR").unwrap_or_else(|_| DEFAULT_DEV_SEEDS_DIR.into()),
            private_dir: var("PRIVATE_DIR").unwrap_or_else(|_| DEFAULT_PRIVATE_DIR.into()),
            session_secret,
            cookie_secure: var("COOKIE_SECURE").is_ok_and(|v| v == "true"),
            analysis_interval_secs: var("ANALYSIS_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            anthropic_api_key: var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            moderation_model: var("MODERATION_MODEL")
                .unwrap_or_else(|_| DEFAULT_MODERATION_MODEL.into()),
            tagging_model: var("TAGGING_MODEL").unwrap_or_else(|_| DEFAULT_TAGGING_MODEL.into()),
            llm_daily_budget_usd: var("LLM_DAILY_BUDGET_USD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LLM_DAILY_BUDGET_USD),
            extraction_model: var("EXTRACTION_MODEL")
                .unwrap_or_else(|_| DEFAULT_EXTRACTION_MODEL.into()),
            llm_extraction_daily_budget_usd: var("LLM_EXTRACTION_DAILY_BUDGET_USD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LLM_EXTRACTION_DAILY_BUDGET_USD),
            collector_contact: var("COLLECTOR_CONTACT")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            webauthn_rp_id: var("WEBAUTHN_RP_ID").unwrap_or_else(|_| DEFAULT_WEBAUTHN_RP_ID.into()),
            webauthn_origin: var("WEBAUTHN_ORIGIN")
                .unwrap_or_else(|_| DEFAULT_WEBAUTHN_ORIGIN.into()),
            trust_forwarded_for: var("TRUST_FORWARDED_FOR").is_ok_and(|v| v == "true"),
        })
    }
}

impl Config {
    /// 本番か（パスキーのドメインが localhost でない）。開発用の seed を入れない判断などに使う
    pub fn is_production(&self) -> bool {
        self.webauthn_rp_id != "localhost"
    }

    /// API サーバーを起動する前の確かめ。設定の食い違いがあれば起動しない（フェーズ7b）
    pub fn check_for_serving(&self) -> anyhow::Result<()> {
        let https = self.webauthn_origin.starts_with("https://");
        if https && !self.cookie_secure {
            anyhow::bail!(
                "WEBAUTHN_ORIGIN が https なのに COOKIE_SECURE が true ではありません（Cookie が HTTPS に限られなくなる）"
            );
        }
        if self.is_production() && !https {
            anyhow::bail!(
                "本番（WEBAUTHN_RP_ID が localhost 以外）では WEBAUTHN_ORIGIN を https にしてください"
            );
        }
        Ok(())
    }
}

/// 環境変数（空や空白だけなら未設定と同じ）
fn var(name: &str) -> Result<String, env::VarError> {
    match env::var(name) {
        Ok(v) if v.trim().is_empty() => Err(env::VarError::NotPresent),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(rp_id: &str, origin: &str, cookie_secure: bool) -> Config {
        Config {
            webauthn_rp_id: rp_id.into(),
            webauthn_origin: origin.into(),
            cookie_secure,
            ..Default::default()
        }
    }

    #[test]
    fn serving_requires_consistent_https_settings() {
        // 開発（localhost・http）
        assert!(
            config("localhost", "http://localhost:3000", false)
                .check_for_serving()
                .is_ok()
        );
        // 本番
        assert!(
            config("example.jp", "https://example.jp", true)
                .check_for_serving()
                .is_ok()
        );
        // https なのに Cookie が Secure でない
        assert!(
            config("example.jp", "https://example.jp", false)
                .check_for_serving()
                .is_err()
        );
        assert!(
            config("localhost", "https://localhost:8443", false)
                .check_for_serving()
                .is_err()
        );
        // 本番のドメインなのに http
        assert!(
            config("example.jp", "http://example.jp", false)
                .check_for_serving()
                .is_err()
        );
        assert!(!config("localhost", "https://localhost:8443", true).is_production());
    }
}
