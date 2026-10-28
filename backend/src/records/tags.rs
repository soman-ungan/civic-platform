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

/// 分野のタグと地区のタグ。`municipality_id` があれば、地区はその自治体のものだけ。
/// 地区のタグは、初めて付けるときに地区の一覧から作られる（`records::topics::set_tag`）ので、
/// まだ使われていない地区も `districts` から出す（id は `district-<地区 ID>`）
pub async fn list(
    pool: &SqlitePool,
    municipality_id: Option<&str>,
) -> Result<Vec<Tag>, sqlx::Error> {
    let mut tags: Vec<Tag> =
        sqlx::query_as("SELECT id, kind, name FROM tags WHERE kind = 'field' ORDER BY id")
            .fetch_all(pool)
            .await?;
    let districts: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, municipality_id, name FROM districts ORDER BY municipality_id, id",
    )
    .fetch_all(pool)
    .await?;
    tags.extend(
        districts
            .into_iter()
            .filter(|(_, m, _)| municipality_id.is_none_or(|want| want == m))
            .map(|(id, _, name)| Tag {
                id: format!("district-{id}"),
                kind: "district".into(),
                name,
            }),
    );
    Ok(tags)
}
