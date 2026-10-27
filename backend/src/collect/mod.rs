// SPDX-License-Identifier: AGPL-3.0-only

//! 行政資料の収集（`docs/design/collection.md`）。
//!
//! 収集は「エージェント」にせず、`collector_sources` の設定に従う決まった手順のコードにする。
//! 自治体・サイトごとの違いはアダプタ（`Collector` の実装）と `config` に閉じ込める。
//! 取得はすべて `fetch::Fetcher` を通す（robots.txt・間隔・条件付き取得・大きさの上限）。

pub mod fetch;
pub mod robots;

use std::time::Duration;

use chrono::{NaiveDate, Utc};
use serde_json::Value;
use sqlx::SqlitePool;

use fetch::Fetcher;

/// `min_interval_secs` の既定（1サイトあたりの接続の間隔）
const DEFAULT_MIN_INTERVAL_SECS: u64 = 5;
/// 規約を確かめてから、これを過ぎたら警告する（規約の変更に気づくため）
const TERMS_RECHECK_DAYS: i64 = 365;

/// `collector_sources` の1行
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Source {
    pub id: String,
    pub municipality_id: String,
    pub kind: String,
    pub adapter: String,
    pub base_url: String,
    pub config: String,
    pub is_enabled: bool,
    pub last_crawled_at: Option<String>,
}

impl Source {
    pub fn config_json(&self) -> anyhow::Result<Value> {
        serde_json::from_str(&self.config).map_err(|e| {
            anyhow::anyhow!("収集先 {} の config が JSON ではありません: {e}", self.id)
        })
    }
}

/// 収集の実行の条件
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// 取得と読み取りだけをして、DB に保存しない
    pub dry_run: bool,
    /// この年度だけを処理する
    pub fiscal_year: Option<i32>,
}

/// 収集の結果（CLI に出す）
#[derive(Debug, Default)]
pub struct Report {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

/// 収集のアダプタ。`collector_sources.adapter` の名前で選ぶ
// CLI から順に呼ぶだけで、別のタスクに渡さないので Send は要らない
#[allow(async_fn_in_trait)]
pub trait Collector {
    async fn collect(
        &self,
        pool: &SqlitePool,
        fetcher: &Fetcher,
        source: &Source,
        options: &Options,
    ) -> anyhow::Result<Report>;
}

pub async fn sources(pool: &SqlitePool) -> Result<Vec<Source>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, municipality_id, kind, adapter, base_url, config, is_enabled, last_crawled_at
         FROM collector_sources ORDER BY municipality_id, kind, id",
    )
    .fetch_all(pool)
    .await
}

/// 規約の確認の記録（`terms_url`・`terms_checked_at`）。無ければ取得しない。
/// 確認から1年を過ぎていたら、警告を返す
pub fn check_terms(config: &Value, today: NaiveDate) -> Result<Option<String>, String> {
    let url = config.get("terms_url").and_then(Value::as_str);
    let checked = config.get("terms_checked_at").and_then(Value::as_str);
    let (Some(_), Some(checked)) = (url, checked) else {
        return Err("利用規約の確認の記録（terms_url・terms_checked_at）がありません".into());
    };
    let checked = NaiveDate::parse_from_str(checked, "%Y-%m-%d")
        .map_err(|_| format!("terms_checked_at が日付（YYYY-MM-DD）ではありません: {checked}"))?;
    let days = (today - checked).num_days();
    Ok((days > TERMS_RECHECK_DAYS).then(|| {
        format!(
            "利用規約を確かめてから {days} 日たっています。規約が変わっていないか確かめてください"
        )
    }))
}

/// 収集先の接続の間隔（`min_interval_secs`）
pub fn min_interval(config: &Value) -> Duration {
    Duration::from_secs(
        config
            .get("min_interval_secs")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_MIN_INTERVAL_SECS),
    )
}

/// 今日の日付（UTC）。規約の確認の期限に使う
pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn terms_record_is_required() {
        assert!(check_terms(&json!({}), day("2026-10-27")).is_err());
        assert!(
            check_terms(
                &json!({"terms_url": "https://example.jp/"}),
                day("2026-10-27")
            )
            .is_err()
        );
        let ok = json!({"terms_url": "https://example.jp/", "terms_checked_at": "2026-10-27"});
        assert_eq!(check_terms(&ok, day("2026-10-27")), Ok(None));
    }

    #[test]
    fn old_terms_check_warns() {
        let c = json!({"terms_url": "https://example.jp/", "terms_checked_at": "2025-10-01"});
        assert!(check_terms(&c, day("2026-10-27")).unwrap().is_some());
    }
}
