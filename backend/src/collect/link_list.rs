// SPDX-License-Identifier: AGPL-3.0-only

//! 一覧ページに PDF のリンクが並ぶ形式（`html_link_list`）。いまは「当初予算の概要」に使う。
//!
//! 自治体ごとの違い（一覧ページの URL・リンクの文字の形）は `config` に置く:
//! - `list_url`: 一覧ページ（無ければ `base_url`）
//! - `link_pattern`: 取り込むリンクの文字の正規表現（例: `当初予算の概要`）
//! - `from_fiscal_year`: この年度より前は取り込まない（AI の費用を抑えるため）
//!
//! 年度はリンクの文字の「令和8年度」などから読む。PDF は読み終えたら捨てる（原本を保持しない）。

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;
use sqlx::SqlitePool;

use super::fetch::{self, Fetched, Fetcher};
use super::settlement_card::{decode_html, fiscal_year_of};
use super::{Collector, Options, Report, Source};
use crate::budget;
use crate::extraction::Extraction;
use crate::llm::Llm;

/// 当初予算の概要の取り込み。PDF を AI で読むので、`Llm` を持つ
pub struct BudgetOverview<'a> {
    pub llm: &'a Llm,
    pub model: &'a str,
}

/// 一覧ページのリンク（URL, リンクの文字）
pub fn links(html: &str, base: &reqwest::Url) -> Vec<(String, String)> {
    static A: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)<a[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap());
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
    A.captures_iter(html)
        .filter_map(|c| {
            let url = base.join(&c[1]).ok()?.to_string();
            let text = TAG.replace_all(&c[2], "").trim().to_owned();
            Some((url, text))
        })
        .collect()
}

/// リンクの文字から年度を読む（「当初予算の概要(令和8年度)[PDF：3.61MB]」→ 2026）
pub fn fiscal_year_in(text: &str) -> Option<i32> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(令和|平成)(元|\d{1,2})年度").unwrap());
    fiscal_year_of(&normalize_digits(RE.find(text)?.as_str()))
}

/// 全角の数字を半角にする
fn normalize_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap_or(c),
            _ => c,
        })
        .collect()
}

impl Collector for BudgetOverview<'_> {
    async fn collect(
        &self,
        pool: &SqlitePool,
        fetcher: &Fetcher,
        source: &Source,
        options: &Options,
    ) -> anyhow::Result<Report> {
        let config = source.config_json()?;
        let list_url = config
            .get("list_url")
            .and_then(Value::as_str)
            .unwrap_or(&source.base_url)
            .to_owned();
        let pattern = Regex::new(
            config
                .get("link_pattern")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("収集先 {}: link_pattern がありません", source.id)
                })?,
        )?;
        let from_year = config
            .get("from_fiscal_year")
            .and_then(Value::as_i64)
            .map(|y| y as i32);

        let base = reqwest::Url::parse(&list_url)?;
        let html = decode_html(&fetcher.get(&list_url).await?);
        let mut targets: Vec<(i32, String)> = links(&html, &base)
            .into_iter()
            .filter(|(_, text)| pattern.is_match(text))
            .filter_map(|(url, text)| Some((fiscal_year_in(&text)?, url)))
            .filter(|(y, _)| {
                from_year.is_none_or(|f| *y >= f) && options.fiscal_year.is_none_or(|f| f == *y)
            })
            .collect();
        targets.sort();
        targets.dedup_by_key(|(y, _)| *y);
        if targets.is_empty() {
            // 一覧ページの形が変わったとみなして止める（自動で設定を直さない）
            anyhow::bail!(
                "取り込むリンクが見つかりません。一覧ページと link_pattern を確かめてください: {list_url}"
            );
        }

        let mut report = Report::default();
        for (year, url) in targets {
            let Fetched::Body { bytes, cache } =
                fetcher.get_if_changed(pool, &source.id, &url).await?
            else {
                report
                    .lines
                    .push(format!("{year}年度: 前回から変わっていません"));
                continue;
            };
            let published: Option<String> = sqlx::query_scalar(
                "SELECT status FROM budgets WHERE municipality_id = $1 AND fiscal_year = $2 AND status = 'published'",
            )
            .bind(&source.municipality_id)
            .bind(year)
            .fetch_optional(pool)
            .await?;
            if published.is_some() {
                // 公開済みの予算は、運営者が確かめずに置き換えない（import budget --again で作り直す）
                report.warnings.push(format!(
                    "{year}年度: 資料が更新されましたが、公開済みなので置き換えません（{url}）"
                ));
                continue;
            }
            let input = budget::Input {
                municipality_id: source.municipality_id.clone(),
                fiscal_year: year,
                url: Some(url.clone()),
                collector_source_id: Some(source.id.clone()),
                pdf: bytes,
            };
            let extraction = Extraction::Api {
                llm: self.llm,
                model: self.model,
            };
            let checked = budget::extract(pool, &extraction, &input).await?;
            if options.dry_run {
                report.lines.push(format!(
                    "{year}年度（--dry-run のため保存していません）\n{}",
                    budget::render(&checked, Some(&url))
                ));
                continue;
            }
            budget::save_draft(pool, &input, &checked, false, extraction.method()).await?;
            fetch::remember(pool, &cache).await?;
            report.lines.push(format!(
                "{year}年度: 下書きを作りました（事業 {} 件、捨てたもの {} 件）。確認: budget show <slug> {year}",
                checked.projects.len(),
                checked.notes.len()
            ));
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_links_and_years() {
        // 大田原市の一覧ページの実物の形
        let html = r#"<a href="file_contents/R8yosangaiyou.pdf">当初予算の概要(令和8年度)[PDF：3.61MB]</a>
<a href="file_contents/R8yosan.pdf">令和8年度大田原市一般会計特別会計予算書[PDF：51.2MB]</a>
<a href="file_contents/yosannogaiyou31.pdf">当初予算の概要(平成31年度)[PDF:1MB]</a>"#;
        let base = reqwest::Url::parse("https://www.city.ohtawara.tochigi.jp/docs/2023051700017/")
            .unwrap();
        let found = links(html, &base);
        assert_eq!(found.len(), 3);
        assert_eq!(
            found[0].0,
            "https://www.city.ohtawara.tochigi.jp/docs/2023051700017/file_contents/R8yosangaiyou.pdf"
        );
        assert_eq!(fiscal_year_in(&found[0].1), Some(2026));
        assert_eq!(fiscal_year_in(&found[2].1), Some(2019));
        assert_eq!(fiscal_year_in("令和８年度当初予算の概要"), Some(2026));
        let pattern = Regex::new("当初予算の概要").unwrap();
        assert_eq!(found.iter().filter(|(_, t)| pattern.is_match(t)).count(), 2);
    }
}
