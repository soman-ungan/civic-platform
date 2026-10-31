// SPDX-License-Identifier: AGPL-3.0-only

//! まちのお金。決算（総務省の決算カード）と予算（自治体の当初予算の概要）。金額は千円、1人あたりは円。
//! 決算の出典は公共データ利用規約（第1.0版）に従い、画面で「総務省 市町村決算カードを加工して作成」と示す。
//! 予算は数字と事業の名前だけを返し、説明は原本の PDF のページへのリンクにする（市の著作物のため）

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::interface::api::AppState;
use crate::interface::api::error::ApiError;
use crate::interface::api::municipalities;

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
    /// settlement: 決算 / budget: 当初予算（決算がまだ無い年度だけ。補正を含まないので決算より小さく出る）
    pub kind: &'static str,
    /// 予算の年度は、それ以前で最新の人口
    pub population: Option<i64>,
    /// 目的別歳出（千円）
    pub expenditure_purpose: BTreeMap<String, Option<i64>>,
}

#[derive(Debug, Serialize)]
pub struct YearInfo {
    pub fiscal_year: i32,
    /// 決算（決算カード）があるか
    pub settlement: bool,
    /// 公開済みの予算があるか
    pub budget: bool,
}

/// ある年度の決算（決算カード）
#[derive(Debug, Serialize)]
pub struct Settlement {
    pub population: Option<i64>,
    pub revenue_total: Option<i64>,
    pub expenditure_total: Option<i64>,
    pub revenue: Vec<Amount>,
    pub expenditure_purpose: Vec<Amount>,
    pub expenditure_nature: Vec<Amount>,
    pub indicators: Vec<Indicator>,
    pub source: Source,
}

/// ある年度の当初予算（公開済みのものだけ）
#[derive(Debug, Serialize)]
pub struct BudgetView {
    /// 一般会計の歳出の合計（千円）
    pub total: i64,
    /// 1人あたりの換算に使った人口と、その年度（予算の年度の人口はまだ無いので、それ以前で最新のもの）
    pub population: Option<i64>,
    pub population_year: Option<i32>,
    pub revenue: Vec<Amount>,
    pub expenditure_purpose: Vec<Amount>,
    pub expenditure_nature: Vec<Amount>,
    pub projects: Vec<BudgetProject>,
    pub source: BudgetSource,
}

#[derive(Debug, Serialize)]
pub struct BudgetSource {
    pub title: String,
    /// 原本の PDF
    pub url: Option<String>,
}

/// 主な事業。説明の文章は持たない（原本の PDF のページへリンクする）
#[derive(Debug, Serialize)]
pub struct BudgetProject {
    pub name: String,
    /// 千円
    pub amount: i64,
    pub per_capita: Option<i64>,
    /// new: 新規 / expanded: 拡充
    pub flag: Option<String>,
    /// その自治体での政策・分野の名前（無い自治体もある）
    pub plan_label: Option<String>,
    pub page: i64,
    pub tags: Vec<ProjectTag>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectTag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Finances {
    /// 決算か予算のある年度（古い順）
    pub years: Vec<YearInfo>,
    pub fiscal_year: i32,
    pub settlement: Option<Settlement>,
    pub budget: Option<BudgetView>,
    /// 推移（目的別歳出。古い順）。決算の年度と、決算がまだ無く予算だけがある年度
    pub trend: Vec<TrendYear>,
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

/// 公開済みの予算のある年度
async fn budget_years(pool: &SqlitePool, municipality_id: &str) -> Result<Vec<i32>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT fiscal_year FROM budgets WHERE municipality_id = $1 AND status = 'published' ORDER BY fiscal_year",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?)
}

/// その年度以前で最新の人口（予算の年度の人口はまだ無いため）
async fn latest_population(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
) -> Result<Option<(i32, i64)>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT fiscal_year, population FROM municipality_stats
         WHERE municipality_id = $1 AND fiscal_year <= $2 ORDER BY fiscal_year DESC LIMIT 1",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?)
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

/// GET /api/municipalities/{slug}/finances?year= — 年度を省くと最新（予算だけの年度を含む）
pub async fn finances(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<FinancesQuery>,
) -> Result<Json<Finances>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let settlement_years = years(&state.pool, &municipality.id).await?;
    let budget_years = budget_years(&state.pool, &municipality.id).await?;
    let mut all: Vec<i32> = settlement_years
        .iter()
        .chain(&budget_years)
        .copied()
        .collect();
    all.sort();
    all.dedup();
    let fiscal_year = match query.year {
        Some(y) if all.contains(&y) => y,
        Some(_) => return Err(ApiError::NotFound),
        None => *all.last().ok_or(ApiError::NotFound)?,
    };

    let rows: Vec<ItemRow> = sqlx::query_as(
        "SELECT fiscal_year, category, item_key, value, unit FROM settlement_items
         WHERE municipality_id = $1 ORDER BY fiscal_year, category, position",
    )
    .bind(&municipality.id)
    .fetch_all(&state.pool)
    .await?;
    let settlement = if settlement_years.contains(&fiscal_year) {
        Some(settlement(&state.pool, &municipality.id, fiscal_year, &rows).await?)
    } else {
        None
    };
    let budget = if budget_years.contains(&fiscal_year) {
        budget(&state.pool, &municipality.id, fiscal_year).await?
    } else {
        None
    };

    let mut trend = Vec::new();
    for &year in &all {
        if settlement_years.contains(&year) {
            trend.push(TrendYear {
                fiscal_year: year,
                kind: "settlement",
                population: population(&state.pool, &municipality.id, year).await?,
                expenditure_purpose: rows
                    .iter()
                    .filter(|r| r.fiscal_year == year && r.category == "expenditure_purpose")
                    .map(|r| (r.item_key.clone(), r.value))
                    .collect(),
            });
        } else {
            let items: Vec<(String, Option<i64>)> = sqlx::query_as(
                "SELECT i.item_key, i.value FROM budget_items i JOIN budgets b ON b.id = i.budget_id
                 WHERE b.municipality_id = $1 AND b.fiscal_year = $2 AND b.status = 'published'
                   AND i.category = 'expenditure_purpose'
                 ORDER BY i.position",
            )
            .bind(&municipality.id)
            .bind(year)
            .fetch_all(&state.pool)
            .await?;
            trend.push(TrendYear {
                fiscal_year: year,
                kind: "budget",
                population: latest_population(&state.pool, &municipality.id, year)
                    .await?
                    .map(|(_, p)| p),
                expenditure_purpose: items.into_iter().collect(),
            });
        }
    }

    Ok(Json(Finances {
        years: all
            .iter()
            .map(|&y| YearInfo {
                fiscal_year: y,
                settlement: settlement_years.contains(&y),
                budget: budget_years.contains(&y),
            })
            .collect(),
        fiscal_year,
        settlement,
        budget,
        trend,
    }))
}

async fn settlement(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
    rows: &[ItemRow],
) -> Result<Settlement, ApiError> {
    let population = population(pool, municipality_id, fiscal_year).await?;
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
    Ok(Settlement {
        source: source(pool, municipality_id, fiscal_year).await?,
        population,
        revenue_total,
        expenditure_total,
        revenue,
        expenditure_purpose,
        expenditure_nature,
        indicators,
    })
}

/// 公開済みの予算。無ければ None
async fn budget(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
) -> Result<Option<BudgetView>, ApiError> {
    let row: Option<(String, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT b.id, b.total, d.title, d.url FROM budgets b JOIN documents d ON d.id = b.document_id
         WHERE b.municipality_id = $1 AND b.fiscal_year = $2 AND b.status = 'published'",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?;
    let Some((id, total, title, url)) = row else {
        return Ok(None);
    };
    let (population_year, population) = latest_population(pool, municipality_id, fiscal_year)
        .await?
        .unzip();
    let items: Vec<ItemRow> = sqlx::query_as(
        "SELECT $2 AS fiscal_year, category, item_key, value, 'thousand_yen' AS unit
         FROM budget_items WHERE budget_id = $1 ORDER BY category, position",
    )
    .bind(&id)
    .bind(fiscal_year)
    .fetch_all(pool)
    .await?;
    let of = |category: &str| -> Vec<&ItemRow> {
        items.iter().filter(|r| r.category == category).collect()
    };
    let projects = projects(pool, &id, population, None).await?;
    Ok(Some(BudgetView {
        total,
        population,
        population_year,
        revenue: amounts(&of("revenue"), population).0,
        expenditure_purpose: amounts(&of("expenditure_purpose"), population).0,
        expenditure_nature: amounts(&of("expenditure_nature"), population).0,
        projects,
        source: BudgetSource { title, url },
    }))
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    amount: i64,
    flag: Option<String>,
    plan_label: Option<String>,
    page: i64,
}

/// その予算の主な事業（金額の大きい順）。
/// `topic_id` を渡すと、そのトピックの分野タグを持つ事業だけにする
async fn projects(
    pool: &SqlitePool,
    budget_id: &str,
    population: Option<i64>,
    topic_id: Option<&str>,
) -> Result<Vec<BudgetProject>, ApiError> {
    let rows: Vec<ProjectRow> = sqlx::query_as(
        "SELECT p.id, p.name, p.amount, p.flag, p.plan_label, p.page FROM budget_projects p
         WHERE p.budget_id = $1
           AND (CAST($2 AS TEXT) IS NULL OR EXISTS (
                SELECT 1 FROM budget_project_tags bt JOIN topic_tags tt ON tt.tag_id = bt.tag_id
                WHERE bt.project_id = p.id AND tt.topic_id = $2))
         ORDER BY p.amount DESC, p.position",
    )
    .bind(budget_id)
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let tags: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT bt.project_id, g.id, g.name FROM budget_project_tags bt
         JOIN tags g ON g.id = bt.tag_id JOIN budget_projects p ON p.id = bt.project_id
         WHERE p.budget_id = $1 ORDER BY g.id",
    )
    .bind(budget_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| BudgetProject {
            tags: tags
                .iter()
                .filter(|(pid, _, _)| *pid == r.id)
                .map(|(_, id, name)| ProjectTag {
                    id: id.clone(),
                    name: name.clone(),
                })
                .collect(),
            per_capita: per_capita(Some(r.amount), population),
            name: r.name,
            amount: r.amount,
            flag: r.flag,
            plan_label: r.plan_label,
            page: r.page,
        })
        .collect())
}

/// トピックの詳細に出す「この分野の市のお金」。トピックの分野タグに対応する目的別歳出（決算）と、
/// 同じ分野の主な事業（予算）。年度の決まりは `topic_years`
#[derive(Debug, Serialize)]
pub struct TopicFinances {
    /// 決算の年度。対応する決算が無ければ null（`items` も空）
    pub fiscal_year: Option<i32>,
    pub population: Option<i64>,
    pub items: Vec<Amount>,
    pub source: Option<Source>,
    /// この分野の主な事業（その年度の公開済みの予算。分野タグが重なるもの、金額の大きい順に5件まで）
    pub budget: Option<TopicBudget>,
}

#[derive(Debug, Serialize)]
pub struct TopicBudget {
    pub fiscal_year: i32,
    pub projects: Vec<BudgetProject>,
    /// 対象の事業の数（5件を超える分は「まちのお金」で見る）
    pub project_count: usize,
    pub source: BudgetSource,
}

/// トピックに出す主な事業の数
const TOPIC_PROJECTS: usize = 5;

pub async fn for_topic(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
) -> Result<Option<TopicFinances>, ApiError> {
    let held_on: Option<String> = sqlx::query_scalar(
        "SELECT d.held_on FROM topic_sources ts JOIN documents d ON d.id = ts.document_id
         WHERE ts.topic_id = $1 AND d.held_on IS NOT NULL ORDER BY d.held_on LIMIT 1",
    )
    .bind(topic_id)
    .fetch_optional(pool)
    .await?;
    let budget_year = held_on.as_deref().and_then(budget_year_of_meeting);
    let years = years(pool, municipality_id).await?;
    // 議会のトピックは、会議の時点で分かっていた最新の決算（予算の年度より前）。市民の提案は最新
    let fiscal_year = match budget_year {
        Some(budget_year) => years.iter().rev().find(|y| **y < budget_year).copied(),
        None => years.last().copied(),
    };
    let budget = topic_budget(pool, municipality_id, topic_id, budget_year).await?;
    let mut items = Vec::new();
    let mut population = None;
    let mut source_info = None;
    if let Some(fiscal_year) = fiscal_year {
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
        if !rows.is_empty() {
            population = self::population(pool, municipality_id, fiscal_year).await?;
            let total = total.unwrap_or(0);
            items = rows
                .into_iter()
                .map(|(key, value)| Amount {
                    per_capita: per_capita(value, population),
                    share: value.filter(|_| total > 0).map(|v| v as f64 / total as f64),
                    key,
                    value,
                })
                .collect();
            source_info = Some(source(pool, municipality_id, fiscal_year).await?);
        }
    }
    if items.is_empty() && budget.is_none() {
        return Ok(None);
    }
    Ok(Some(TopicFinances {
        fiscal_year: source_info.as_ref().and(fiscal_year),
        population,
        items,
        source: source_info,
        budget,
    }))
}

/// 会議の日（YYYY-MM-DD）から、その会議に対応する予算の年度: 会議の日の年。1月の会議だけ前年。
/// 第1回定例会（2〜3月）は翌年度の当初予算を審議する会で、4〜12月の会はその年度の会なので、こうなる
pub fn budget_year_of_meeting(held_on: &str) -> Option<i32> {
    let year: i32 = held_on.get(0..4)?.parse().ok()?;
    let month: u32 = held_on.get(5..7)?.parse().ok()?;
    Some(if month == 1 { year - 1 } else { year })
}

/// トピックの分野の主な事業。`fiscal_year` があればその年度の予算だけ（無ければ出さない。年度のずれた数字を並べない）、
/// 無ければ公開済みの最新の予算
async fn topic_budget(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
    fiscal_year: Option<i32>,
) -> Result<Option<TopicBudget>, ApiError> {
    let row: Option<(String, i32, String, Option<String>)> = sqlx::query_as(
        "SELECT b.id, b.fiscal_year, d.title, d.url FROM budgets b JOIN documents d ON d.id = b.document_id
         WHERE b.municipality_id = $1 AND b.status = 'published'
           AND (CAST($2 AS INTEGER) IS NULL OR b.fiscal_year = $2)
         ORDER BY b.fiscal_year DESC LIMIT 1",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?;
    let Some((id, fiscal_year, title, url)) = row else {
        return Ok(None);
    };
    let population = latest_population(pool, municipality_id, fiscal_year)
        .await?
        .map(|(_, p)| p);
    let mut projects = projects(pool, &id, population, Some(topic_id)).await?;
    if projects.is_empty() {
        return Ok(None);
    }
    let project_count = projects.len();
    projects.truncate(TOPIC_PROJECTS);
    Ok(Some(TopicBudget {
        fiscal_year,
        projects,
        project_count,
        source: BudgetSource { title, url },
    }))
}
