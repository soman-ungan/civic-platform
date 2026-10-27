// SPDX-License-Identifier: AGPL-3.0-only

//! トピックの下書きの確認と公開（docs/plans/2026-10-27-phase5a-minutes.md Step 4）。
//! 会議録から AI が作ったトピックは下書き（draft）で保存され、運営者が内容・出典・発言者を
//! 確かめてから公開する。認証（フェーズ6）までは CLI から使う。

use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct DraftSummary {
    pub id: String,
    pub municipality_id: String,
    pub title: String,
    pub created_at: String,
}

/// 下書きの一覧（古い順）
pub async fn list(pool: &SqlitePool) -> Result<Vec<DraftSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, municipality_id, title, created_at FROM topics
         WHERE status = 'draft' ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct SourceLine {
    pub document_title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
    pub utterance_no: i64,
    pub speaker_label: String,
    pub quote: String,
}

/// 運営者が確かめるための、トピックの全体
#[derive(Debug)]
pub struct DraftDetail {
    pub id: String,
    pub status: String,
    pub title: String,
    pub description: String,
    pub sources: Vec<SourceLine>,
    pub statements: Vec<String>,
    pub tags: Vec<String>,
}

pub async fn show(pool: &SqlitePool, topic_id: &str) -> Result<Option<DraftDetail>, sqlx::Error> {
    let Some((id, status, title, description)): Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id, status, title, description FROM topics WHERE id = $1")
            .bind(topic_id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(None);
    };
    let sources = sqlx::query_as(
        "SELECT d.title AS document_title, d.held_on, d.url, s.utterance_no, s.speaker_label, s.quote
         FROM topic_sources s JOIN documents d ON d.id = s.document_id
         WHERE s.topic_id = $1 ORDER BY s.position",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let statements = sqlx::query_scalar(
        "SELECT body FROM statements WHERE topic_id = $1 AND status = 'approved' ORDER BY id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let tags = sqlx::query_scalar(
        "SELECT g.name FROM topic_tags tt JOIN tags g ON g.id = tt.tag_id
         WHERE tt.topic_id = $1 ORDER BY g.kind, g.name",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    Ok(Some(DraftDetail {
        id,
        status,
        title,
        description,
        sources,
        statements,
        tags,
    }))
}

impl DraftDetail {
    pub fn render(&self) -> String {
        let mut out = format!(
            "{}  [{}]\n# {}\n\n{}\n\n## 出典\n",
            self.id, self.status, self.title, self.description
        );
        for s in &self.sources {
            out.push_str(&format!(
                "- {}（{}） {} 発言{}: 「{}」\n  {}\n",
                s.document_title,
                s.held_on.as_deref().unwrap_or("日付なし"),
                s.speaker_label,
                s.utterance_no,
                s.quote,
                s.url.as_deref().unwrap_or("URL なし"),
            ));
        }
        out.push_str("\n## 最初の意見\n");
        for body in &self.statements {
            out.push_str(&format!("- {body}\n"));
        }
        out.push_str(&format!(
            "\n## タグ\n{}\n",
            if self.tags.is_empty() {
                "（まだ付いていない）".to_owned()
            } else {
                self.tags.join("、")
            }
        ));
        out
    }
}

/// 公開する・保管する（公開をやめる）。下書き・公開済みのトピックが対象。見つからなければ false
pub async fn set_status(
    pool: &SqlitePool,
    topic_id: &str,
    status: &str,
) -> Result<bool, sqlx::Error> {
    debug_assert!(["published", "archived"].contains(&status));
    let result = sqlx::query("UPDATE topics SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(topic_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
