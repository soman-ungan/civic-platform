// SPDX-License-Identifier: AGPL-3.0-only

//! 行政資料の収集（`docs/design/collection.md`）。
//!
//! 収集は「エージェント」にせず、`collector_sources` の設定に従う決まった手順のコードにする。
//! 自治体・サイトごとの違いはアダプタ（`Collector` の実装）と `config` に閉じ込める。
//! 取得はすべて `fetch::Fetcher` を通す（robots.txt・間隔・条件付き取得・大きさの上限）。
//!
//! 取得（`fetch`）は原本を置き場（`store`）に置くところまで。読み取りと保存は、決算カードは
//! `settlement_card::import`、予算の概要は `budget`（AI で読むので `extract` を挟む）が行う。

pub mod fetch;
pub mod link_list;
pub mod robots;
pub mod settlement_card;
pub mod store;

use std::time::Duration;

use chrono::{NaiveDate, Utc};
use serde_json::Value;
use sqlx::SqlitePool;

use fetch::Fetcher;
use store::Store;

/// `min_interval_secs` の既定（1サイトあたりの接続の間隔）
const DEFAULT_MIN_INTERVAL_SECS: u64 = 5;
/// 規約を確かめてから、これを過ぎたら警告する（規約の変更に気づくため）
const TERMS_RECHECK_DAYS: i64 = 365;

/// `collector_sources` の1行
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Source {
    pub id: String,
    pub municipality_id: String,
    /// 自治体の URL の名前（置き場のディレクトリに使う）
    pub slug: String,
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

/// 取得の条件
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// 取得して確かめるだけで、置き場に置かない（取得の記録も残さない）
    pub dry_run: bool,
    /// この年度だけを処理する
    pub fiscal_year: Option<i32>,
}

/// 取得・取り込みの結果（CLI に出す）
#[derive(Debug, Default)]
pub struct Report {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

/// 取得のアダプタ。`collector_sources.adapter` の名前で選ぶ。
/// 変わった原本だけを置き場に置く（変わっていなければ置かない）
// CLI から順に呼ぶだけで、別のタスクに渡さないので Send は要らない
#[allow(async_fn_in_trait)]
pub trait Collector {
    async fn fetch(
        &self,
        pool: &SqlitePool,
        fetcher: &Fetcher,
        source: &Source,
        options: &Options,
        store: &Store,
    ) -> anyhow::Result<Report>;
}

pub async fn sources(pool: &SqlitePool) -> Result<Vec<Source>, sqlx::Error> {
    sqlx::query_as(
        "SELECT s.id, s.municipality_id, m.slug, s.kind, s.adapter, s.base_url, s.config, s.is_enabled, s.last_crawled_at
         FROM collector_sources s JOIN municipalities m ON m.id = s.municipality_id
         ORDER BY s.municipality_id, s.kind, s.id",
    )
    .fetch_all(pool)
    .await
}

/// 収集先の `from_fiscal_year`（この年度より前は取らない）を変える。config のほかの項目はそのまま。
/// 遡って取り込むと決めたとき（AI の費用と、共同作業の手間を見て）に使う。
/// 無い収集先なら Ok(false)
pub async fn set_from_fiscal_year(pool: &SqlitePool, id: &str, year: i32) -> anyhow::Result<bool> {
    let Some((config,)): Option<(String,)> =
        sqlx::query_as("SELECT config FROM collector_sources WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(false);
    };
    let mut value: Value = serde_json::from_str(&config)
        .map_err(|e| anyhow::anyhow!("収集先 {id} の config が JSON ではありません: {e}"))?;
    let Some(map) = value.as_object_mut() else {
        anyhow::bail!("収集先 {id} の config が JSON のオブジェクトではありません");
    };
    map.insert("from_fiscal_year".into(), Value::from(year));
    sqlx::query("UPDATE collector_sources SET config = $1 WHERE id = $2")
        .bind(value.to_string())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(true)
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
fn min_interval(config: &Value) -> Duration {
    Duration::from_secs(
        config
            .get("min_interval_secs")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_MIN_INTERVAL_SECS),
    )
}

/// 1つの収集先から取得する。無効な収集先・規約の確認の記録が無い収集先は動かさない。
/// `contact` は運営者の連絡先（`COLLECTOR_CONTACT`。User-Agent に入れる）
pub async fn fetch_source(
    pool: &SqlitePool,
    source: &Source,
    contact: &str,
    options: &Options,
    store: &Store,
) -> anyhow::Result<Report> {
    if !source.is_enabled {
        anyhow::bail!("収集先 {} は無効です（is_enabled = false）", source.id);
    }
    let config = source.config_json()?;
    let warning = check_terms(&config, Utc::now().date_naive())
        .map_err(|e| anyhow::anyhow!("収集先 {}: {e}", source.id))?;
    let fetcher = Fetcher::new(contact, min_interval(&config))?;

    let mut report = match source.adapter.as_str() {
        "soumu_settlement_card" => {
            settlement_card::SettlementCard
                .fetch(pool, &fetcher, source, options, store)
                .await?
        }
        "html_link_list" if source.kind == "budget" => {
            link_list::BudgetOverview
                .fetch(pool, &fetcher, source, options, store)
                .await?
        }
        other => anyhow::bail!(
            "収集先 {}: 自動で取得できないアダプタです: {other}",
            source.id
        ),
    };
    report.warnings.extend(warning);
    if !options.dry_run {
        sqlx::query("UPDATE collector_sources SET last_crawled_at = $1 WHERE id = $2")
            .bind(crate::shared::util::now_iso())
            .bind(&source.id)
            .execute(pool)
            .await?;
    }
    Ok(report)
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
