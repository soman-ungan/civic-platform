// SPDX-License-Identifier: AGPL-3.0-only

//! まちのお金（総務省の決算カード）。金額は千円、1人あたりは円。
//! 出典は公共データ利用規約（第1.0版）に従い、画面で「総務省 市町村決算カードを加工して作成」と示す

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::AppState;
use crate::error::ApiError;
use crate::routes::municipalities;

#[derive(Debug, Serialize)]
pub struct Amount {
    pub key: String,
    /// 千円。原表が「-」なら null
    pub value: Option<i64>,
    /// 市民1人あたり（円）
    pub per_capita: Option<i64>,
    /// 区分の合計に占める割合（0〜1）
    pub share: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct Indicator {
    pub key: String,
    pub value: Option<f64>,
    /// thousand_yen（千円）/ ratio（比率・指数。原表のまま。比率は％）
    pub unit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Source {
    pub title: String,
    /// 決算カードの年度のページ
    pub page_url: Option<String>,
    pub excel_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TrendYear {
    pub fiscal_year: i32,
    pub population: Option<i64>,
    /// 目的別歳出（千円）
    pub expenditure_purpose: BTreeMap<String, Option<i64>>,
}

#[derive(Debug, Serialize)]
pub struct Finances {
    /// データのある年度（古い順）
    pub years: Vec<i32>,
    pub fiscal_year: i32,
    pub population: Option<i64>,
    pub revenue_total: Option<i64>,
    pub expenditure_total: Option<i64>,
    pub revenue: Vec<Amount>,
    pub expenditure_purpose: Vec<Amount>,
    pub expenditure_nature: Vec<Amount>,
    pub indicators: Vec<Indicator>,
    pub trend: Vec<TrendYear>,
    pub source: Source,
}

#[derive(Debug, Deserialize)]
pub struct FinancesQuery {
    pub year: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
struct ItemRow {
    fiscal_year: i32,
    category: String,
    item_key: String,
    value: Option<i64>,
    unit: String,
}

async fn years(pool: &SqlitePool, municipality_id: &str) -> Result<Vec<i32>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT fiscal_year FROM settlement_items WHERE municipality_id = $1 ORDER BY fiscal_year",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?)
}

async fn population(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
) -> Result<Option<i64>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT population FROM municipality_stats WHERE municipality_id = $1 AND fiscal_year = $2",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?)
}

async fn source(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
) -> Result<Source, ApiError> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT d.title, d.url FROM documents d
         WHERE d.id = (SELECT MIN(i.document_id) FROM settlement_items i
                       WHERE i.municipality_id = $1 AND i.fiscal_year = $2)",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?;
    let page_url: Option<String> = sqlx::query_scalar(
        "SELECT source_url FROM municipality_stats WHERE municipality_id = $1 AND fiscal_year = $2",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?;
    let (title, excel_url) = row.unwrap_or_else(|| ("市町村決算カード".into(), None));
    Ok(Source {
        title,
        page_url,
        excel_url,
    })
}

/// 1人あたり（円）。金額は千円
fn per_capita(value: Option<i64>, population: Option<i64>) -> Option<i64> {
    let (v, p) = (value?, population.filter(|p| *p > 0)?);
    Some(((v as f64 * 1000.0) / p as f64).round() as i64)
}

fn amounts(rows: &[&ItemRow], population: Option<i64>) -> (Vec<Amount>, Option<i64>) {
    let total: i64 = rows.iter().map(|r| r.value.unwrap_or(0)).sum();
    let amounts = rows
        .iter()
        .map(|r| Amount {
            key: r.item_key.clone(),
            value: r.value,
            per_capita: per_capita(r.value, population),
            share: r
                .value
                .filter(|_| total > 0)
                .map(|v| v as f64 / total as f64),
        })
        .collect();
    (amounts, (!rows.is_empty()).then_some(total))
}

/// GET /api/municipalities/{slug}/finances?year= — 年度を省くと最新
pub async fn finances(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<FinancesQuery>,
) -> Result<Json<Finances>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let years = years(&state.pool, &municipality.id).await?;
    let fiscal_year = match query.year {
        Some(y) if years.contains(&y) => y,
        Some(_) => return Err(ApiError::NotFound),
        None => *years.last().ok_or(ApiError::NotFound)?,
    };

    let rows: Vec<ItemRow> = sqlx::query_as(
        "SELECT fiscal_year, category, item_key, value, unit FROM settlement_items
         WHERE municipality_id = $1 ORDER BY fiscal_year, category, position",
    )
    .bind(&municipality.id)
    .fetch_all(&state.pool)
    .await?;
    let population = population(&state.pool, &municipality.id, fiscal_year).await?;
    let of = |category: &str| -> Vec<&ItemRow> {
        rows.iter()
            .filter(|r| r.fiscal_year == fiscal_year && r.category == category)
            .collect()
    };
    let (revenue, revenue_total) = amounts(&of("revenue"), population);
    let (expenditure_purpose, expenditure_total) = amounts(&of("expenditure_purpose"), population);
    let (expenditure_nature, _) = amounts(&of("expenditure_nature"), population);
    let indicators = of("indicator")
        .into_iter()
        .map(|r| Indicator {
            key: r.item_key.clone(),
            value: r.value.map(|v| {
                if r.unit == "milli" {
                    v as f64 / 1000.0
                } else {
                    v as f64
                }
            }),
            unit: if r.unit == "milli" {
                "ratio"
            } else {
                "thousand_yen"
            },
        })
        .collect();

    let mut trend = Vec::new();
    for &year in &years {
        trend.push(TrendYear {
            fiscal_year: year,
            population: self::population(&state.pool, &municipality.id, year).await?,
            expenditure_purpose: rows
                .iter()
                .filter(|r| r.fiscal_year == year && r.category == "expenditure_purpose")
                .map(|r| (r.item_key.clone(), r.value))
                .collect(),
        });
    }

    Ok(Json(Finances {
        source: source(&state.pool, &municipality.id, fiscal_year).await?,
        years,
        fiscal_year,
        population,
        revenue_total,
        expenditure_total,
        revenue,
        expenditure_purpose,
        expenditure_nature,
        indicators,
        trend,
    }))
}

/// トピックの詳細に出す「この分野の市のお金」。トピックの分野タグに対応する目的別歳出（最新の年度）
#[derive(Debug, Serialize)]
pub struct TopicFinances {
    pub fiscal_year: i32,
    pub population: Option<i64>,
    pub items: Vec<Amount>,
    pub source: Source,
}

pub async fn for_topic(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
) -> Result<Option<TopicFinances>, ApiError> {
    let Some(&fiscal_year) = years(pool, municipality_id).await?.last() else {
        return Ok(None);
    };
    let total: Option<i64> = sqlx::query_scalar(
        "SELECT SUM(value) FROM settlement_items
         WHERE municipality_id = $1 AND fiscal_year = $2 AND category = 'expenditure_purpose'",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_one(pool)
    .await?;
    let rows: Vec<(String, Option<i64>)> = sqlx::query_as(
        "SELECT i.item_key, i.value FROM settlement_items i
         WHERE i.municipality_id = $1 AND i.fiscal_year = $2 AND i.category = 'expenditure_purpose'
           AND EXISTS (
               SELECT 1 FROM settlement_item_tags m
               JOIN topic_tags tt ON tt.tag_id = m.tag_id
               WHERE m.category = i.category AND m.item_key = i.item_key AND tt.topic_id = $3)
         ORDER BY i.position",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let population = population(pool, municipality_id, fiscal_year).await?;
    let total = total.unwrap_or(0);
    let items = rows
        .into_iter()
        .map(|(key, value)| Amount {
            per_capita: per_capita(value, population),
            share: value.filter(|_| total > 0).map(|v| v as f64 / total as f64),
            key,
            value,
        })
        .collect();
    Ok(Some(TopicFinances {
        source: source(pool, municipality_id, fiscal_year).await?,
        fiscal_year,
        population,
        items,
    }))
}
