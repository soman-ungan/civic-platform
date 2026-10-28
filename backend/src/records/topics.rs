// SPDX-License-Identifier: AGPL-3.0-only

//! トピックの一覧・確認・手直し・公開（CLI の list / show / edit / publish / archive topic、
//! add / edit / remove statement、add / remove tag。docs/plans/2026-10-27-phase5a-minutes.md Step 4）。
//! 会議録から AI が作ったトピックは下書き（draft）で保存され、運営者が内容・出典・発言者を
//! 確かめてから公開する。認証（フェーズ6）までは CLI から使う。

use sqlx::SqlitePool;

use crate::shared::status::Publication;

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
pub struct TopicSummary {
    pub id: String,
    pub created_at: String,
    pub slug: String,
    pub status: String,
    pub title: String,
}

/// すべてのトピック（下書き・公開済み・取り下げ。自治体ごとに古い順）
pub async fn list_all(pool: &SqlitePool) -> Result<Vec<TopicSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT t.id, t.created_at, m.slug, t.status, t.title
         FROM topics t JOIN municipalities m ON m.id = t.municipality_id
         ORDER BY m.slug, t.created_at",
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
    /// (意見の ID, 本文, 状態)
    pub statements: Vec<(String, String, String)>,
    /// 意見の ID → 根拠の発言（「発言12 〇〇議員」）。会議録から作った意見だけ
    pub basis: std::collections::HashMap<String, Vec<String>>,
    /// (タグの ID, 名前, 付けたのは ai / admin)
    pub tags: Vec<(String, String, String)>,
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
    let statements = sqlx::query_as(
        "SELECT id, body, status FROM statements
         WHERE topic_id = $1 AND status IN ('approved', 'pending', 'flagged') ORDER BY id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let basis_rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT ss.statement_id, ss.utterance_no, ss.speaker_label FROM statement_sources ss
         JOIN statements s ON s.id = ss.statement_id
         WHERE s.topic_id = $1 ORDER BY ss.utterance_no",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let mut basis: std::collections::HashMap<String, Vec<String>> = Default::default();
    for (sid, no, speaker) in basis_rows {
        basis
            .entry(sid)
            .or_default()
            .push(format!("発言{no} {speaker}"));
    }
    let tags = sqlx::query_as(
        "SELECT g.id, g.name, tt.assigned_by FROM topic_tags tt JOIN tags g ON g.id = tt.tag_id
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
        basis,
        tags,
    }))
}

impl DraftDetail {
    pub fn render(&self) -> String {
        let mut out = format!(
            "{}  [{}]\n# {}\n\n{}\n（説明 {} 字）\n\n## 出典\n",
            self.id,
            self.status,
            self.title,
            self.description,
            self.description.chars().count()
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
        for (id, body, status) in &self.statements {
            let mark = if status == "approved" {
                String::new()
            } else {
                format!("[{status}] ")
            };
            out.push_str(&format!("- {mark}{body}\n  {id}\n"));
            if let Some(basis) = self.basis.get(id) {
                out.push_str(&format!("  根拠: {}\n", basis.join("、")));
            }
        }
        out.push_str("\n## タグ\n");
        if self.tags.is_empty() {
            out.push_str("（まだ付いていない）\n");
        }
        for (id, name, by) in &self.tags {
            out.push_str(&format!("- {name}（{id}、{by}）\n"));
        }
        out
    }
}

/// 公開する・保管する（公開をやめる）。下書き・公開済みのトピックが対象。見つからなければ false
pub async fn set_status(
    pool: &SqlitePool,
    topic_id: &str,
    status: Publication,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE topics SET status = $1 WHERE id = $2")
        .bind(status.as_str())
        .bind(topic_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// 運営者の手直しの失敗
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("見つかりません: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// 意見の本文の確かめ（投稿と同じく1〜140文字）
fn check_body(body: &str) -> Result<String, EditError> {
    let body = body.trim();
    let chars = body.chars().count();
    if !(1..=140).contains(&chars) {
        return Err(EditError::Invalid(format!(
            "意見は1〜140文字にしてください（{chars}文字）"
        )));
    }
    Ok(body.to_owned())
}

/// タイトル・説明を直す
pub async fn edit_topic(
    pool: &SqlitePool,
    topic_id: &str,
    title: Option<&str>,
    description: Option<&str>,
) -> Result<(), EditError> {
    if let Some(title) = title.map(str::trim)
        && title.is_empty()
    {
        return Err(EditError::Invalid("タイトルが空です".into()));
    }
    let result = sqlx::query(
        "UPDATE topics SET title = COALESCE($1, title), description = COALESCE($2, description)
         WHERE id = $3",
    )
    .bind(title.map(str::trim))
    .bind(description.map(str::trim))
    .bind(topic_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(EditError::NotFound(topic_id.to_owned()));
    }
    Ok(())
}

/// 最初の意見を足す（運営者の意見として、投稿者なし・承認済み）
pub async fn add_statement(
    pool: &SqlitePool,
    topic_id: &str,
    body: &str,
) -> Result<String, EditError> {
    let body = check_body(body)?;
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM topics WHERE id = $1")
        .bind(topic_id)
        .fetch_optional(pool)
        .await?;
    let Some(status) = status else {
        return Err(EditError::NotFound(topic_id.to_owned()));
    };
    // 下書きに足すものは最初の意見。公開したあとに足すものは、最初の意見としては扱わない
    let is_seed = status == Publication::Draft.as_str();
    let id = crate::shared::util::new_id();
    sqlx::query(
        "INSERT INTO statements (id, topic_id, user_id, body, status, created_at, is_seed)
         VALUES ($1, $2, NULL, $3, 'approved', $4, $5)",
    )
    .bind(&id)
    .bind(topic_id)
    .bind(&body)
    .bind(crate::shared::util::now_iso())
    .bind(is_seed)
    .execute(pool)
    .await?;
    Ok(id)
}

/// 意見の本文を直す。投票がある意見は直せない（投票した人が見た内容と変わってしまうため）
pub async fn edit_statement(
    pool: &SqlitePool,
    statement_id: &str,
    body: &str,
) -> Result<(), EditError> {
    let body = check_body(body)?;
    let votes: Option<i64> = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM votes v WHERE v.statement_id = s.id)
         FROM statements s WHERE s.id = $1",
    )
    .bind(statement_id)
    .fetch_optional(pool)
    .await?;
    match votes {
        None => return Err(EditError::NotFound(statement_id.to_owned())),
        Some(n) if n > 0 => {
            return Err(EditError::Invalid(format!(
                "投票が{n}件ある意見は直せません。取り下げて（statement remove）新しく足してください"
            )));
        }
        Some(_) => {}
    }
    sqlx::query("UPDATE statements SET body = $1 WHERE id = $2")
        .bind(&body)
        .bind(statement_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// タグを付ける（運営者のタグとして）・外す
pub async fn set_tag(
    pool: &SqlitePool,
    topic_id: &str,
    tag_id: &str,
    add: bool,
) -> Result<(), EditError> {
    if add {
        // 地区タグ（district-<地区 ID>）がまだ無ければ、地区の一覧から作る
        if let Some(district_id) = tag_id.strip_prefix("district-") {
            sqlx::query(
                "INSERT INTO tags (id, kind, name, district_id)
                 SELECT $1, 'district', name, id FROM districts WHERE id = $2
                 ON CONFLICT DO NOTHING",
            )
            .bind(tag_id)
            .bind(district_id)
            .execute(pool)
            .await?;
        }
        // 地区タグは、そのトピックの自治体の地区に限る
        let ok: Option<String> = sqlx::query_scalar(
            "SELECT g.id FROM tags g, topics t
             WHERE g.id = $1 AND t.id = $2
               AND (g.kind = 'field'
                    OR EXISTS (SELECT 1 FROM districts d
                               WHERE d.id = g.district_id AND d.municipality_id = t.municipality_id))",
        )
        .bind(tag_id)
        .bind(topic_id)
        .fetch_optional(pool)
        .await?;
        if ok.is_none() {
            return Err(EditError::Invalid(format!(
                "トピック {topic_id} に付けられるタグではありません: {tag_id}"
            )));
        }
        // AI が付けていたものは運営者のタグにする
        sqlx::query(
            "INSERT INTO topic_tags (topic_id, tag_id, assigned_by) VALUES ($1, $2, 'admin')
             ON CONFLICT (topic_id, tag_id) DO UPDATE SET assigned_by = 'admin'",
        )
        .bind(topic_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    } else {
        let result = sqlx::query("DELETE FROM topic_tags WHERE topic_id = $1 AND tag_id = $2")
            .bind(topic_id)
            .bind(tag_id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(EditError::NotFound(format!("{topic_id} の {tag_id}")));
        }
        // 外したタグを AI が付け直さないよう、タグ付けを済みにしておく
        sqlx::query("UPDATE topics SET tagged_at = COALESCE(tagged_at, $1) WHERE id = $2")
            .bind(crate::shared::util::now_iso())
            .bind(topic_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}
