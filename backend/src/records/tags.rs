// SPDX-License-Identifier: AGPL-3.0-only

//! タグの語彙（分野）と地区（CLI の list tag）。トピックに付ける・外すのは `records::topics::set_tag`。
//! 語彙は seed で管理する（data/seeds/020_field_tags.sql）

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    /// field / district
    pub kind: String,
    pub name: String,
}

/// 分野のタグと地区のタグ。`municipality_id` があれば、地区はその自治体のものだけ
pub async fn list(
    pool: &SqlitePool,
    municipality_id: Option<&str>,
) -> Result<Vec<Tag>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        tag: Tag,
        municipality_id: Option<String>,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT t.id, t.kind, t.name, d.municipality_id
         FROM tags t LEFT JOIN districts d ON d.id = t.district_id
         ORDER BY t.kind DESC, d.municipality_id, t.id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|r| match (municipality_id, &r.municipality_id) {
            (Some(want), Some(m)) => want == m,
            _ => true,
        })
        .map(|r| r.tag)
        .collect())
}
