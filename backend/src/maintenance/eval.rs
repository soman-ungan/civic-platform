// SPDX-License-Identifier: AGPL-3.0-only

//! モデレーションとタグ付けの評価（docs/plans/2026-10-27-phase4-ai.md Step 4）。
//! `evals/*.jsonl` を実際の API で判定し、正解率・取り違えの内訳・費用・遅延を出す。
//! 費用がかかるので、実行は運営者の判断で行う（`cargo run -- eval ...`）。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Deserialize;
use sqlx::SqlitePool;

use crate::process::moderation::Moderation;
use crate::process::tagging::{Tagger, classify};
use crate::shared::llm::Purpose;
use crate::shared::util::now_iso;

/// モデレーションの評価データの1件
#[derive(Debug, Clone, Deserialize)]
pub struct ModerationCase {
    pub id: String,
    pub body: String,
    /// approved / rejected / flagged
    pub expected: String,
    /// 何を確かめるケースか
    #[serde(default)]
    pub note: String,
}

/// タグ付けの評価データの1件。タグは名前で書く
#[derive(Debug, Clone, Deserialize)]
pub struct TaggingCase {
    pub id: String,
    pub municipality_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub fields: Vec<String>,
    #[serde(default)]
    pub districts: Vec<String>,
    /// 何を確かめるケースか
    #[serde(default)]
    pub note: String,
}

pub fn load<T: for<'de> Deserialize<'de>>(path: &Path) -> anyhow::Result<Vec<T>> {
    let text = std::fs::read_to_string(path)?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(n, line)| {
            serde_json::from_str(line)
                .map_err(|e| anyhow::anyhow!("{}:{}: {e}", path.display(), n + 1))
        })
        .collect()
}

/// 評価の間に記録された費用（100万分の1ドル）
async fn eval_cost_since(pool: &SqlitePool, since: &str) -> Result<i64, sqlx::Error> {
    let (sum,): (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(cost_micro_usd) FROM llm_usage WHERE purpose = 'eval' AND created_at >= $1",
    )
    .bind(since)
    .fetch_one(pool)
    .await?;
    Ok(sum.unwrap_or(0))
}

fn usd(micro: i64) -> String {
    format!("${:.4}", micro as f64 / 1_000_000.0)
}

fn median(mut v: Vec<Duration>) -> Duration {
    if v.is_empty() {
        return Duration::ZERO;
    }
    v.sort();
    v[v.len() / 2]
}

/// モデレーションの評価結果
#[derive(Debug, Default)]
pub struct ModerationReport {
    pub total: usize,
    pub correct: usize,
    /// (正解, 判定) → 件数
    pub confusion: BTreeMap<(String, String), usize>,
    /// 外れたケース (id, 正解, 判定, note)
    pub misses: Vec<(String, String, String, String)>,
    pub cost_micro_usd: i64,
    pub median_latency: Duration,
}

pub async fn run_moderation(
    pool: &SqlitePool,
    moderation: &Moderation,
    cases: &[ModerationCase],
) -> anyhow::Result<ModerationReport> {
    let since = now_iso();
    let mut report = ModerationReport::default();
    let mut latencies = Vec::new();
    for case in cases {
        let started = Instant::now();
        let review = moderation.review_as(pool, &case.body, Purpose::Eval).await;
        latencies.push(started.elapsed());
        let actual = review.verdict.status().as_str().to_owned();
        report.total += 1;
        if actual == case.expected {
            report.correct += 1;
        } else {
            report.misses.push((
                case.id.clone(),
                case.expected.clone(),
                actual.clone(),
                case.note.clone(),
            ));
        }
        *report
            .confusion
            .entry((case.expected.clone(), actual))
            .or_default() += 1;
    }
    report.cost_micro_usd = eval_cost_since(pool, &since).await?;
    report.median_latency = median(latencies);
    Ok(report)
}

impl ModerationReport {
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "正解 {}/{}（{:.1}%）",
            self.correct,
            self.total,
            100.0 * self.correct as f64 / self.total.max(1) as f64
        );
        // 特に重要な取り違え
        let missed_harm = self.count("rejected", "approved");
        let over_block = self.count("approved", "rejected");
        let _ = writeln!(out, "見逃し（非公開にすべきものを公開）: {missed_harm}");
        let _ = writeln!(out, "行き過ぎ（公開すべきものを非公開）: {over_block}");
        let _ = writeln!(out, "内訳（正解 → 判定: 件数）");
        for ((expected, actual), n) in &self.confusion {
            let _ = writeln!(out, "  {expected} → {actual}: {n}");
        }
        let _ = writeln!(
            out,
            "費用 {}（1件あたり {}）、遅延の中央値 {:.1} 秒",
            usd(self.cost_micro_usd),
            usd(self.cost_micro_usd / self.total.max(1) as i64),
            self.median_latency.as_secs_f64()
        );
        if !self.misses.is_empty() {
            let _ = writeln!(out, "外れたケース");
            for (id, expected, actual, note) in &self.misses {
                let _ = writeln!(out, "  {id}: {expected} → {actual}  {note}");
            }
        }
        out
    }

    fn count(&self, expected: &str, actual: &str) -> usize {
        self.confusion
            .get(&(expected.to_owned(), actual.to_owned()))
            .copied()
            .unwrap_or(0)
    }
}

/// 適合率と再現率を数えるための集計
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
}

impl Counts {
    fn add(&mut self, expected: &BTreeSet<String>, actual: &BTreeSet<String>) {
        self.true_positive += expected.intersection(actual).count();
        self.false_positive += actual.difference(expected).count();
        self.false_negative += expected.difference(actual).count();
    }

    fn line(&self, label: &str) -> String {
        let precision =
            self.true_positive as f64 / (self.true_positive + self.false_positive).max(1) as f64;
        let recall =
            self.true_positive as f64 / (self.true_positive + self.false_negative).max(1) as f64;
        format!(
            "{label}: 適合率 {:.1}%、再現率 {:.1}%",
            100.0 * precision,
            100.0 * recall
        )
    }
}

/// タグ付けの評価結果
#[derive(Debug, Default)]
pub struct TaggingReport {
    pub total: usize,
    /// 分野と地区が完全に一致した件数
    pub exact: usize,
    pub fields: Counts,
    pub districts: Counts,
    pub errors: usize,
    /// 一致しなかったケース (id, 正解, 判定, note)
    pub misses: Vec<(String, String, String, String)>,
    pub cost_micro_usd: i64,
    pub median_latency: Duration,
}

pub async fn run_tagging(
    pool: &SqlitePool,
    tagger: &Tagger,
    cases: &[TaggingCase],
) -> anyhow::Result<TaggingReport> {
    // ID → 名前（結果を名前で比べるため）
    let field_names: BTreeMap<String, String> =
        sqlx::query_as::<_, (String, String)>("SELECT id, name FROM tags WHERE kind = 'field'")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
    let district_names: BTreeMap<String, String> =
        sqlx::query_as::<_, (String, String)>("SELECT id, name FROM districts")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();

    let since = now_iso();
    let mut report = TaggingReport::default();
    let mut latencies = Vec::new();
    for case in cases {
        report.total += 1;
        let started = Instant::now();
        let result = classify(
            pool,
            tagger,
            &case.municipality_id,
            &case.title,
            &case.description,
            Purpose::Eval,
        )
        .await;
        latencies.push(started.elapsed());
        let classified = match result {
            Ok(c) => c,
            Err(err) => {
                tracing::warn!(id = case.id, error = %err, "eval case failed");
                report.errors += 1;
                continue;
            }
        };
        let names = |ids: &[String], map: &BTreeMap<String, String>| -> BTreeSet<String> {
            ids.iter().filter_map(|id| map.get(id).cloned()).collect()
        };
        let actual_fields = names(&classified.field_ids, &field_names);
        let actual_districts = names(&classified.district_ids, &district_names);
        let expected_fields: BTreeSet<String> = case.fields.iter().cloned().collect();
        let expected_districts: BTreeSet<String> = case.districts.iter().cloned().collect();
        report.fields.add(&expected_fields, &actual_fields);
        report.districts.add(&expected_districts, &actual_districts);
        if actual_fields == expected_fields && actual_districts == expected_districts {
            report.exact += 1;
        } else {
            report.misses.push((
                case.id.clone(),
                format!("{expected_fields:?} {expected_districts:?}"),
                format!("{actual_fields:?} {actual_districts:?}"),
                case.note.clone(),
            ));
        }
    }
    report.cost_micro_usd = eval_cost_since(pool, &since).await?;
    report.median_latency = median(latencies);
    Ok(report)
}

impl TaggingReport {
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "完全一致 {}/{}（{:.1}%）、エラー {}",
            self.exact,
            self.total,
            100.0 * self.exact as f64 / self.total.max(1) as f64,
            self.errors
        );
        let _ = writeln!(out, "{}", self.fields.line("分野"));
        let _ = writeln!(out, "{}", self.districts.line("地区"));
        let _ = writeln!(
            out,
            "費用 {}（1件あたり {}）、遅延の中央値 {:.1} 秒",
            usd(self.cost_micro_usd),
            usd(self.cost_micro_usd / self.total.max(1) as i64),
            self.median_latency.as_secs_f64()
        );
        if !self.misses.is_empty() {
            let _ = writeln!(out, "一致しなかったケース（正解 / 判定）");
            for (id, expected, actual, note) in &self.misses {
                let _ = writeln!(out, "  {id}: {expected} / {actual}  {note}");
            }
        }
        out
    }
}
