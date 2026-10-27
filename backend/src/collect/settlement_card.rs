// SPDX-License-Identifier: AGPL-3.0-only

//! 総務省の市町村決算カード（`soumu_settlement_card`）。全市町村が同じ形式なので、1つのアダプタで扱う。
//!
//! 1. 決算カードの一覧ページから、年度ごとのページを探す（リンクの文字「令和5年度市町村決算カード」）
//! 2. 年度のページから、その自治体の都道府県の Excel を探す（北海道は複数に分かれている）
//! 3. Excel の中から、団体コードで自分の自治体のシートを探す（シートの名前は年度で違う）
//! 4. 見出しの文字で値の位置を探して読み取り、合計で検算してから保存する
//!
//! セルの位置は年度で行がずれるので固定しない。見出しの右にある最初の値を、その見出しの値とする。
//! 原本（Excel）は保存しない。数値と出典（`documents`）だけを残す。
//! 出典の表示は公共データ利用規約（第1.0版）に従う（`docs/design/collection.md`）。

use std::io::Cursor;
use std::sync::LazyLock;

use calamine::{Data, Reader, Xlsx};
use regex::Regex;
use serde_json::Value;
use sqlx::SqlitePool;

use super::fetch::{self, Fetched, Fetcher};
use super::{Collector, Options, Report, Source};

/// Excel のある最初の年度（平成27年度）
const DEFAULT_FROM_FISCAL_YEAR: i32 = 2015;
/// 見出しと、それが属する表の見出し（「目的別歳出の状況」など）の列の差の上限
const SECTION_COLUMN_SPAN: u32 = 8;

// ===== Excel のセル =====

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Number(f64),
}

/// シートの、空でないセルの一覧（行・列の順）
#[derive(Debug, Default)]
pub struct Grid {
    cells: Vec<(u32, u32, Cell)>,
}

impl Grid {
    /// 文字は空白を除き、全角の英数字・記号を半角にしてから持つ（「財 産 収 入」「平成２７年度」）
    pub fn new(cells: impl IntoIterator<Item = (u32, u32, Cell)>) -> Self {
        let mut cells: Vec<(u32, u32, Cell)> = cells
            .into_iter()
            .filter_map(|(r, c, cell)| match cell {
                Cell::Text(t) => {
                    let t = normalize(&t);
                    (!t.is_empty()).then_some((r, c, Cell::Text(t)))
                }
                n => Some((r, c, n)),
            })
            .collect();
        cells.sort_by_key(|(r, c, _)| (*r, *c));
        Self { cells }
    }

    pub fn from_range(range: &calamine::Range<Data>) -> Self {
        let (r0, c0) = range.start().unwrap_or((0, 0));
        Self::new(range.used_cells().filter_map(|(r, c, d)| {
            let cell = match d {
                Data::String(s) => Cell::Text(s.clone()),
                Data::Float(f) => Cell::Number(*f),
                Data::Int(i) => Cell::Number(*i as f64),
                _ => return None,
            };
            Some((r0 + r as u32, c0 + c as u32, cell))
        }))
    }

    fn text_at(&self, label: &str) -> impl Iterator<Item = (u32, u32)> + '_ {
        let label = normalize(label);
        self.cells
            .iter()
            .filter_map(move |(r, c, cell)| match cell {
                Cell::Text(t) if *t == label => Some((*r, *c)),
                _ => None,
            })
    }

    fn first_text(&self, label: &str) -> Option<(u32, u32)> {
        self.text_at(label).next()
    }

    /// 同じ行で、`col` より右にある最初のセル
    fn right_of(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells
            .iter()
            .find(|(r, c, _)| *r == row && *c > col)
            .map(|(_, _, cell)| cell)
    }

    /// 同じ行で、`col` より右にある最初の数値（単位の「(%)」は飛ばす）
    fn number_right_of(&self, row: u32, col: u32) -> Option<Option<f64>> {
        self.cells
            .iter()
            .filter(|(r, c, _)| *r == row && *c > col)
            .find_map(|(_, _, cell)| match cell {
                Cell::Text(t) if t == "(%)" => None,
                other => Some(value_of(other)),
            })
            .flatten()
    }

    /// 同じ列で、`row` より下にあるセル（上から順に）
    fn below(&self, row: u32, col: u32) -> impl Iterator<Item = &Cell> {
        self.cells
            .iter()
            .filter(move |(r, c, _)| *c == col && *r > row)
            .map(|(_, _, cell)| cell)
    }
}

/// 値のセルなら `Some`（「-」は該当なしで `Some(None)`）、文字の見出しなら `None`
fn value_of(cell: &Cell) -> Option<Option<f64>> {
    match cell {
        Cell::Number(n) => Some(Some(*n)),
        Cell::Text(t) if t == "-" => Some(None),
        Cell::Text(_) => None,
    }
}

/// 空白（全角を含む）と改行を除き、全角の英数字・記号（U+FF01〜U+FF5E）を半角にする
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .collect()
}

// ===== 決算カードの読み取り =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Revenue,
    ExpenditurePurpose,
    ExpenditureNature,
    Indicator,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Revenue => "revenue",
            Self::ExpenditurePurpose => "expenditure_purpose",
            Self::ExpenditureNature => "expenditure_nature",
            Self::Indicator => "indicator",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    ThousandYen,
    /// 値の1000倍
    Milli,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ThousandYen => "thousand_yen",
            Self::Milli => "milli",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub category: Category,
    pub key: String,
    pub position: i32,
    /// 「-」なら None
    pub value: Option<i64>,
    pub unit: Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// 団体コード（6桁）
    pub code: String,
    pub prefecture: String,
    pub name: String,
    pub fiscal_year: i32,
    /// 住民基本台帳の人口（カードの1行目。調査の年度の1月1日現在）
    pub population: Option<i64>,
    pub items: Vec<Item>,
}

impl Card {
    pub fn total(&self, category: Category) -> Option<i64> {
        self.items
            .iter()
            .filter(|i| i.category == category)
            .map(|i| i.value.unwrap_or(0))
            .reduce(|a, b| a + b)
    }
}

/// シートの団体コード（「都道府県名」「団体名」の見出しの下にある2桁と4桁）。決算カードのシートでなければ None
pub fn sheet_code(grid: &Grid) -> Option<String> {
    let code_below = |label: &str, width: usize| -> Option<String> {
        let (r, c) = grid.first_text(label)?;
        match grid.below(r, c).next()? {
            Cell::Text(t) if t.chars().all(|c| c.is_ascii_digit()) => Some(format!("{t:0>width$}")),
            Cell::Number(n) => Some(format!("{:0width$}", *n as i64)),
            _ => None,
        }
    };
    let code = format!(
        "{}{}",
        code_below("都道府県名", 2)?,
        code_below("団体名", 4)?
    );
    (code.len() == 6).then_some(code)
}

/// 1つのシートを読み取る。形式が想定と違えば、理由を返す
pub fn parse_card(grid: &Grid) -> Result<Card, String> {
    let code = sheet_code(grid).ok_or("団体コードが見つかりません")?;
    let name_below = |label: &str| -> Option<String> {
        let (r, c) = grid.first_text(label)?;
        grid.below(r, c).nth(1).and_then(|cell| match cell {
            Cell::Text(t) => Some(t.clone()),
            Cell::Number(_) => None,
        })
    };
    let prefecture = name_below("都道府県名").ok_or("都道府県名が見つかりません")?;
    let name = name_below("団体名").ok_or("団体名が見つかりません")?;
    let fiscal_year = grid
        .cells
        .iter()
        .find_map(|(_, _, cell)| match cell {
            Cell::Text(t) => fiscal_year_of(t),
            Cell::Number(_) => None,
        })
        .ok_or("年度が見つかりません")?;
    let population = grid.first_text("住民基本台帳人口").and_then(|(r, c)| {
        grid.below(r, c).find_map(|cell| match cell {
            Cell::Number(n) => Some(n.round() as i64),
            Cell::Text(_) => None,
        })
    });

    let header = |label: &str| {
        grid.first_text(label)
            .ok_or_else(|| format!("「{label}」が見つかりません"))
    };
    let revenue = header("歳入の状況")?;
    let nature = header("性質別歳出の状況")?;
    let purpose = header("目的別歳出の状況")?;

    let mut items = Vec::new();
    items.extend(section(
        grid,
        Category::Revenue,
        revenue,
        Some(nature.0),
        "歳入合計",
    )?);
    items.extend(section(
        grid,
        Category::ExpenditureNature,
        nature,
        None,
        "歳出合計",
    )?);
    items.extend(section(
        grid,
        Category::ExpenditurePurpose,
        purpose,
        None,
        "歳出合計",
    )?);
    items.extend(indicators(grid));

    Ok(Card {
        code,
        prefecture,
        name,
        fiscal_year,
        population,
        items,
    })
}

/// 表（歳入・性質別・目的別）の費目。合計の行と同じ列にある見出しのうち、
/// 右に値がある行を費目とする（「内訳」のように右が文字なら費目ではない）。
/// 小計（「(義務的経費計)」など「計」を含むもの）は除く。最後に合計で検算する
fn section(
    grid: &Grid,
    category: Category,
    (header_row, header_col): (u32, u32),
    until_row: Option<u32>,
    total_label: &str,
) -> Result<Vec<Item>, String> {
    let in_section = |r: u32, c: u32| {
        r > header_row
            && until_row.is_none_or(|u| r < u)
            && c.abs_diff(header_col) <= SECTION_COLUMN_SPAN
    };
    let (total_row, label_col) = grid
        .text_at(total_label)
        .find(|(r, c)| in_section(*r, *c))
        .ok_or_else(|| format!("{}の「{total_label}」が見つかりません", category.as_str()))?;
    let total = grid
        .right_of(total_row, label_col)
        .and_then(value_of)
        .flatten()
        .ok_or_else(|| format!("{}の「{total_label}」の値がありません", category.as_str()))?;
    let total = total.round() as i64;

    let mut items = Vec::new();
    for (r, c, cell) in &grid.cells {
        let Cell::Text(label) = cell else { continue };
        if *c != label_col || *r <= header_row || *r >= total_row || label.contains('計') {
            continue;
        }
        let Some(value) = grid.right_of(*r, *c).and_then(value_of) else {
            continue;
        };
        items.push(Item {
            category,
            key: label.clone(),
            position: items.len() as i32 + 1,
            value: value.map(|v| v.round() as i64),
            unit: Unit::ThousandYen,
        });
    }
    let sum: i64 = items.iter().map(|i| i.value.unwrap_or(0)).sum();
    if sum != total {
        return Err(format!(
            "{}の費目の合計（{sum} 千円）が「{total_label}」（{total} 千円）と一致しません",
            category.as_str()
        ));
    }
    Ok(items)
}

/// 財政指標など。見つからない指標は飛ばす（年度によって無いものがあるため）
fn indicators(grid: &Grid) -> Vec<Item> {
    let mut items = Vec::new();
    let mut push = |key: &str, value: Option<f64>, unit: Unit| {
        items.push(Item {
            category: Category::Indicator,
            key: key.to_owned(),
            position: items.len() as i32 + 1,
            value: value.map(|v| match unit {
                Unit::ThousandYen => v.round() as i64,
                Unit::Milli => (v * 1000.0).round() as i64,
            }),
            unit,
        });
    };
    for (label, unit) in [
        ("財政力指数", Unit::Milli),
        ("実質収支比率", Unit::Milli),
        ("実質公債費比率", Unit::Milli),
        ("将来負担比率", Unit::Milli),
        ("地方債現在高", Unit::ThousandYen),
    ] {
        if let Some(value) = grid
            .first_text(label)
            .and_then(|(r, c)| grid.number_right_of(r, c))
        {
            push(label, value, unit);
        }
    }
    // 経常収支比率は、性質別の表の列の見出しにも同じ文字がある。全体の値は最後の見出しの次の行にある
    if let Some((r, c)) = grid.text_at("経常収支比率").last()
        && let Some(value) = grid
            .cells
            .iter()
            .filter(|(rr, cc, _)| *rr == r + 1 && *cc >= c)
            .find_map(|(_, _, cell)| match cell {
                Cell::Number(n) => Some(*n),
                Cell::Text(_) => None,
            })
    {
        push("経常収支比率", Some(value), Unit::Milli);
    }
    items
}

/// 「令和5年度」「平成27年度」「令和元年度」→ 西暦の年度。ちょうどその形の文字だけを読む
pub fn fiscal_year_of(text: &str) -> Option<i32> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(令和|平成)(元|\d{1,2})年度$").unwrap());
    let caps = RE.captures(text)?;
    let n: i32 = match &caps[2] {
        "元" => 1,
        n => n.parse().ok()?,
    };
    Some(match &caps[1] {
        "令和" => 2018 + n,
        _ => 1988 + n,
    })
}

/// Excel の中から、団体コードのシートを探して読み取る。無ければ Ok(None)
pub fn find_card(bytes: &[u8], code: &str) -> Result<Option<Card>, String> {
    let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes))
        .map_err(|e| format!("Excel を開けません: {e}"))?;
    for name in wb.sheet_names() {
        let range = wb
            .worksheet_range(&name)
            .map_err(|e| format!("シート「{name}」を読めません: {e}"))?;
        let grid = Grid::from_range(&range);
        if sheet_code(&grid).as_deref() == Some(code) {
            return parse_card(&grid)
                .map(Some)
                .map_err(|e| format!("シート「{name}」: {e}"));
        }
    }
    Ok(None)
}

// ===== 一覧ページ =====

/// HTML を文字にする。`charset` / `encoding` の指定（総務省は Shift_JIS）に従い、無ければ UTF-8
pub fn decode_html(bytes: &[u8]) -> String {
    static RE: LazyLock<regex::bytes::Regex> = LazyLock::new(|| {
        regex::bytes::Regex::new(r#"(?i)(?:charset|encoding)\s*=\s*["']?([A-Za-z0-9_-]+)"#).unwrap()
    });
    let head = &bytes[..bytes.len().min(2048)];
    let encoding = RE
        .captures(head)
        .and_then(|c| encoding_rs::Encoding::for_label(&c[1]))
        .unwrap_or(encoding_rs::UTF_8);
    encoding.decode(bytes).0.into_owned()
}

/// 一覧ページから、年度ごとのページ（年度, URL）を探す
pub fn year_pages(html: &str, base: &reqwest::Url) -> Vec<(i32, String)> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<a[^>]*href="([^"]+)"[^>]*>\s*((?:令和|平成)(?:元|\d{1,2})年度)市町村決算カード\s*</a>"#)
            .unwrap()
    });
    let mut pages: Vec<(i32, String)> = RE
        .captures_iter(html)
        .filter_map(|c| {
            let year = fiscal_year_of(&c[2])?;
            Some((year, base.join(&c[1]).ok()?.to_string()))
        })
        .collect();
    pages.sort();
    pages.dedup_by_key(|(y, _)| *y);
    pages
}

/// 年度のページから、その都道府県の欄にある Excel の URL を探す（北海道は複数）
pub fn excel_links(html: &str, prefecture: &str, base: &reqwest::Url) -> Vec<String> {
    static TD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<td[^>]*>(.*?)</td>").unwrap());
    static EXCEL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<a[^>]*href="([^"]+)"[^>]*>[^<]*<img[^>]*alt="EXCEL""#).unwrap()
    });
    let marker = format!("■{prefecture}");
    TD.captures_iter(html)
        .filter(|td| td[1].contains(&marker))
        .flat_map(|td| {
            EXCEL
                .captures_iter(&td[1])
                .filter_map(|a| base.join(&a[1]).ok().map(|u| u.to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

// ===== 収集 =====

pub struct SettlementCard;

impl Collector for SettlementCard {
    async fn collect(
        &self,
        pool: &SqlitePool,
        fetcher: &Fetcher,
        source: &Source,
        options: &Options,
    ) -> anyhow::Result<Report> {
        let config = source.config_json()?;
        let index_url = config
            .get("index_url")
            .and_then(Value::as_str)
            .unwrap_or(&source.base_url)
            .to_owned();
        let from_year = config
            .get("from_fiscal_year")
            .and_then(Value::as_i64)
            .map_or(DEFAULT_FROM_FISCAL_YEAR, |y| y as i32);
        let (code, prefecture): (String, String) = sqlx::query_as(
            "SELECT m.id, p.name FROM municipalities m
             JOIN prefectures p ON p.code = m.prefecture_code WHERE m.id = $1",
        )
        .bind(&source.municipality_id)
        .fetch_one(pool)
        .await?;

        let mut report = Report::default();
        let index = reqwest::Url::parse(&index_url)?;
        let pages: Vec<(i32, String)> =
            year_pages(&decode_html(&fetcher.get(&index_url).await?), &index)
                .into_iter()
                .filter(|(y, _)| *y >= from_year && options.fiscal_year.is_none_or(|f| f == *y))
                .collect();
        if pages.is_empty() {
            // 一覧ページの形が変わったとみなして止める（自動で設定を直さない）
            anyhow::bail!(
                "決算カードの年度のページが見つかりません。一覧ページの形が変わっていないか確かめてください: {index_url}"
            );
        }

        for (year, page_url) in pages {
            let page = reqwest::Url::parse(&page_url)?;
            let html = decode_html(&fetcher.get(&page_url).await?);
            let links = excel_links(&html, &prefecture, &page);
            if links.is_empty() {
                report.warnings.push(format!(
                    "{year}年度: {prefecture}の Excel が見つかりません（{page_url}）"
                ));
                continue;
            }
            let mut outcome = None;
            for link in links {
                let Fetched::Body { bytes, cache } =
                    fetcher.get_if_changed(pool, &source.id, &link).await?
                else {
                    outcome.get_or_insert_with(|| "前回から変わっていません".to_owned());
                    continue;
                };
                match find_card(&bytes, &code) {
                    Ok(None) => {
                        if !options.dry_run {
                            fetch::remember(pool, &cache).await?;
                        }
                    }
                    Ok(Some(card)) if card.fiscal_year != year => {
                        report.warnings.push(format!(
                            "{year}年度: Excel の年度（{}年度）がページと違うので保存しません（{link}）",
                            card.fiscal_year
                        ));
                        outcome = Some("保存しませんでした".into());
                        break;
                    }
                    Ok(Some(card)) => {
                        let summary = format!(
                            "{}{} 歳出 {} 千円・人口 {} 人",
                            card.prefecture,
                            card.name,
                            card.total(Category::ExpenditurePurpose).unwrap_or(0),
                            card.population.map_or("-".into(), |p| p.to_string()),
                        );
                        if options.dry_run {
                            outcome =
                                Some(format!("{summary}（--dry-run のため保存していません）"));
                        } else {
                            save(pool, source, &card, &link, &page_url, &cache, bytes.len())
                                .await?;
                            fetch::remember(pool, &cache).await?;
                            outcome = Some(format!("{summary}（保存しました）"));
                        }
                        if card.population.is_none() {
                            report
                                .warnings
                                .push(format!("{year}年度: 人口が見つかりません"));
                        }
                        break;
                    }
                    Err(e) => {
                        // 形式が想定と違う。記録しないので、直してから再び取り込める
                        report.warnings.push(format!("{year}年度: {e}（{link}）"));
                        outcome = Some("保存しませんでした".into());
                        break;
                    }
                }
            }
            let outcome = outcome.unwrap_or_else(|| {
                report.warnings.push(format!(
                    "{year}年度: Excel に団体コード {code} のシートがありません"
                ));
                "見つかりませんでした".into()
            });
            report.lines.push(format!("{year}年度: {outcome}"));
        }
        Ok(report)
    }
}

async fn save(
    pool: &SqlitePool,
    source: &Source,
    card: &Card,
    excel_url: &str,
    page_url: &str,
    cache: &fetch::CacheEntry,
    size: usize,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let title = format!(
        "市町村決算カード {}年度（{}）",
        card.fiscal_year, card.prefecture
    );
    // char_count には、文字数の代わりに Excel の大きさ（バイト）を入れる
    sqlx::query(
        "INSERT INTO documents (id, municipality_id, kind, title, url, sha256, char_count, collector_source_id, imported_at)
         VALUES ($1, $2, 'settlement_card', $3, $4, $5, $6, $7, $8)
         ON CONFLICT (municipality_id, sha256) DO NOTHING",
    )
    .bind(crate::util::new_id())
    .bind(&source.municipality_id)
    .bind(&title)
    .bind(excel_url)
    .bind(&cache.sha256)
    .bind(size as i64)
    .bind(&source.id)
    .bind(crate::util::now_iso())
    .execute(&mut *tx)
    .await?;
    let document_id: String =
        sqlx::query_scalar("SELECT id FROM documents WHERE municipality_id = $1 AND sha256 = $2")
            .bind(&source.municipality_id)
            .bind(&cache.sha256)
            .fetch_one(&mut *tx)
            .await?;

    sqlx::query("DELETE FROM settlement_items WHERE municipality_id = $1 AND fiscal_year = $2")
        .bind(&source.municipality_id)
        .bind(card.fiscal_year)
        .execute(&mut *tx)
        .await?;
    for item in &card.items {
        sqlx::query(
            "INSERT INTO settlement_items (municipality_id, fiscal_year, category, item_key, position, value, unit, document_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&source.municipality_id)
        .bind(card.fiscal_year)
        .bind(item.category.as_str())
        .bind(&item.key)
        .bind(item.position)
        .bind(item.value)
        .bind(item.unit.as_str())
        .bind(&document_id)
        .execute(&mut *tx)
        .await?;
    }
    if let Some(population) = card.population.filter(|p| *p > 0) {
        sqlx::query(
            "INSERT INTO municipality_stats (municipality_id, fiscal_year, population, households, source_url)
             VALUES ($1, $2, $3, NULL, $4)
             ON CONFLICT (municipality_id, fiscal_year) DO UPDATE SET
                 population = excluded.population, source_url = excluded.source_url",
        )
        .bind(&source.municipality_id)
        .bind(card.fiscal_year)
        .bind(population)
        .bind(page_url)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
