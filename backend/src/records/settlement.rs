// SPDX-License-Identifier: AGPL-3.0-only

//! 決算カードの一覧と1年度分（CLI の list / show settlement）。
//! 決算カードは下書きを持たず、取り込んだら公開される（取り込みは `ingest::collect::settlement_card`）

use sqlx::SqlitePool;

/// 1年度分の要約
#[derive(Debug, sqlx::FromRow)]
pub struct YearSummary {
    pub slug: String,
    pub fiscal_year: i32,
    /// 目的別歳出の合計（千円）
    pub expenditure: i64,
    pub population: Option<i64>,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<YearSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT m.slug, s.fiscal_year, SUM(s.value) AS expenditure, st.population
         FROM settlement_items s
         JOIN municipalities m ON m.id = s.municipality_id
         LEFT JOIN municipality_stats st
           ON st.municipality_id = s.municipality_id AND st.fiscal_year = s.fiscal_year
         WHERE s.category = 'expenditure_purpose'
         GROUP BY m.slug, s.fiscal_year, st.population
         ORDER BY m.slug, s.fiscal_year",
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct Item {
    pub category: String,
    pub item_key: String,
    pub value: i64,
    pub unit: String,
}

/// 1年度分の決算カード
#[derive(Debug)]
pub struct Card {
    pub population: Option<i64>,
    /// 原本（Excel）の URL
    pub source_url: Option<String>,
    pub items: Vec<Item>,
}

/// 1年度分を読む。無ければ None
pub async fn load(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
) -> Result<Option<Card>, sqlx::Error> {
    let items: Vec<Item> = sqlx::query_as(
        "SELECT category, item_key, value, unit FROM settlement_items
         WHERE municipality_id = $1 AND fiscal_year = $2
         ORDER BY category, position",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_all(pool)
    .await?;
    if items.is_empty() {
        return Ok(None);
    }
    let source_url: Option<String> = sqlx::query_scalar(
        "SELECT d.url FROM settlement_items s JOIN documents d ON d.id = s.document_id
         WHERE s.municipality_id = $1 AND s.fiscal_year = $2 ORDER BY s.category, s.position",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?
    .flatten();
    let population: Option<i64> = sqlx::query_scalar(
        "SELECT population FROM municipality_stats WHERE municipality_id = $1 AND fiscal_year = $2",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?;
    Ok(Some(Card {
        population,
        source_url,
        items,
    }))
}
