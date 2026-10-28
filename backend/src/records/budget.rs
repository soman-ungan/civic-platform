// SPDX-License-Identifier: AGPL-3.0-only

//! 予算の一覧・読み出し・公開と取り下げ（CLI の list / show / publish / archive budget）。
//! 下書きは API のどこにも出ない。取り込み（読み取り・検査・下書きの保存）は `ingest::budget`

use sqlx::SqlitePool;

use crate::ingest::budget::{Checked, Project};
use crate::shared::status::Publication;
use crate::shared::util::now_iso;

/// 下書き・公開済みの予算の一覧（運営者の確認用）
pub async fn list(pool: &SqlitePool) -> Result<Vec<(String, i32, String, i64)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT m.slug, b.fiscal_year, b.status, b.total FROM budgets b
         JOIN municipalities m ON m.id = b.municipality_id
         ORDER BY m.slug, b.fiscal_year",
    )
    .fetch_all(pool)
    .await
}

/// 公開・取り下げ。見つからなければ false
pub async fn set_status(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
    status: Publication,
) -> Result<bool, sqlx::Error> {
    let published_at = (status == Publication::Published).then(now_iso);
    let n = sqlx::query(
        "UPDATE budgets SET status = $1, published_at = COALESCE($2, published_at)
         WHERE municipality_id = $3 AND fiscal_year = $4",
    )
    .bind(status.as_str())
    .bind(published_at)
    .bind(municipality_id)
    .bind(fiscal_year)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(n > 0)
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

/// 保存済みの予算を `Checked` の形で読む（`show budget`）
pub async fn load(
    pool: &SqlitePool,
    municipality_id: &str,
    fiscal_year: i32,
) -> Result<Option<(String, Checked, Option<String>)>, sqlx::Error> {
    let row: Option<(String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT b.id, b.status, b.total, d.url FROM budgets b JOIN documents d ON d.id = b.document_id
         WHERE b.municipality_id = $1 AND b.fiscal_year = $2",
    )
    .bind(municipality_id)
    .bind(fiscal_year)
    .fetch_optional(pool)
    .await?;
    let Some((id, status, total, url)) = row else {
        return Ok(None);
    };
    let items: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT category, item_key, value FROM budget_items WHERE budget_id = $1 ORDER BY category, position",
    )
    .bind(&id)
    .fetch_all(pool)
    .await?;
    let projects: Vec<ProjectRow> = sqlx::query_as(
        "SELECT id, name, amount, flag, plan_label, page FROM budget_projects WHERE budget_id = $1 ORDER BY position",
    )
    .bind(&id)
    .fetch_all(pool)
    .await?;
    let tags: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.project_id, t.tag_id FROM budget_project_tags t
         JOIN budget_projects p ON p.id = t.project_id WHERE p.budget_id = $1 ORDER BY t.tag_id",
    )
    .bind(&id)
    .fetch_all(pool)
    .await?;
    let category = |c: &str| -> &'static str {
        match c {
            "revenue" => "revenue",
            "expenditure_purpose" => "expenditure_purpose",
            _ => "expenditure_nature",
        }
    };
    Ok(Some((
        status,
        Checked {
            total,
            items: items
                .into_iter()
                .map(|(c, k, v)| (category(&c), k, v))
                .collect(),
            projects: projects
                .into_iter()
                .map(|r| Project {
                    tag_ids: tags
                        .iter()
                        .filter(|(p, _)| *p == r.id)
                        .map(|(_, t)| t.clone())
                        .collect(),
                    name: r.name,
                    amount: r.amount,
                    flag: match r.flag.as_deref() {
                        Some("new") => Some("new"),
                        Some("expanded") => Some("expanded"),
                        _ => None,
                    },
                    plan_label: r.plan_label,
                    page: r.page,
                })
                .collect(),
            notes: Vec::new(),
        },
        url,
    )))
}
