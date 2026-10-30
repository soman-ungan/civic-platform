// SPDX-License-Identifier: AGPL-3.0-only

//! 提言の保存と読み出し（ADR 0005。docs/plans/2026-10-30-opinion-synthesis.md）。
//! 中身（項目）は `process::recommendation` がプログラムで決め、ここでは写しを保存する。
//! 公開した提言は変えない（見出し・前書きを直せるのは下書きだけ）。下書きは運営者の API 以外に出さない

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::shared::limits::{MAX_DESCRIPTION_CHARS, MAX_TITLE_CHARS};
use crate::shared::status::{ItemKind, Publication};
use crate::shared::util::{new_id, now_iso};

/// グループごとの票（提言を作ったときの写し）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupVotes {
    /// A, B, C…（元のトピックの分析での並び）
    pub label: String,
    pub members: usize,
    pub agree: u32,
    pub disagree: u32,
    /// 投票した人数（パスを含む）
    pub seen: u32,
}

/// 提言の項目（合意点か、意見が分かれた点）。数字は分析に含まれた人の票
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub kind: ItemKind,
    pub topic_id: String,
    pub statement_id: String,
    pub body: String,
    pub agree: u32,
    pub disagree: u32,
    pub seen: u32,
    pub groups: Vec<GroupVotes>,
}

/// 直接の元のトピック（作ったときの人数と関わり方の内訳）
pub struct NewTopic {
    pub topic_id: String,
    pub participants: i64,
    /// 関わり方ごとの人数（二次秘匿の済んだもの。隠した区分は None）
    pub residency: BTreeMap<String, Option<i64>>,
}

pub struct NewRecommendation {
    pub municipality_id: String,
    pub consensus_threshold: f64,
    pub divided_threshold: f64,
    pub topics: Vec<NewTopic>,
    pub children: Vec<String>,
    pub items: Vec<Item>,
}

/// 下書きとして保存する。戻り値は提言の ID
pub async fn insert(pool: &SqlitePool, new: &NewRecommendation) -> Result<String, sqlx::Error> {
    let id = new_id();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO recommendations
            (id, municipality_id, consensus_threshold, divided_threshold, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(&new.municipality_id)
    .bind(new.consensus_threshold)
    .bind(new.divided_threshold)
    .bind(now_iso())
    .execute(&mut *tx)
    .await?;
    for (i, t) in new.topics.iter().enumerate() {
        sqlx::query(
            "INSERT INTO recommendation_topics (recommendation_id, topic_id, participants, residency, position)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&id)
        .bind(&t.topic_id)
        .bind(t.participants)
        .bind(serde_json::to_string(&t.residency).expect("JSON にできる"))
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    for (i, child) in new.children.iter().enumerate() {
        sqlx::query(
            "INSERT INTO recommendation_children (parent_id, child_id, position) VALUES ($1, $2, $3)",
        )
        .bind(&id)
        .bind(child)
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    for (i, item) in new.items.iter().enumerate() {
        sqlx::query(
            "INSERT INTO recommendation_items
                (id, recommendation_id, kind, topic_id, statement_id, body, agree, disagree, seen, groups, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(new_id())
        .bind(&id)
        .bind(item.kind.as_str())
        .bind(&item.topic_id)
        .bind(&item.statement_id)
        .bind(&item.body)
        .bind(i64::from(item.agree))
        .bind(i64::from(item.disagree))
        .bind(i64::from(item.seen))
        .bind(serde_json::to_string(&item.groups).expect("JSON にできる"))
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// その自治体の公開済みの提言か（子にできるか）
pub async fn is_published_in(
    pool: &SqlitePool,
    municipality_id: &str,
    id: &str,
) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM recommendations
         WHERE id = $1 AND municipality_id = $2 AND status = 'published'",
    )
    .bind(id)
    .bind(municipality_id)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    kind: String,
    topic_id: String,
    statement_id: String,
    body: String,
    agree: i64,
    disagree: i64,
    seen: i64,
    groups: String,
}

/// 提言の項目（並び順）
pub async fn items(pool: &SqlitePool, id: &str) -> Result<Vec<Item>, sqlx::Error> {
    let rows: Vec<ItemRow> = sqlx::query_as(
        "SELECT kind, topic_id, statement_id, body, agree, disagree, seen, groups
         FROM recommendation_items WHERE recommendation_id = $1 ORDER BY position",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Item {
            kind: ItemKind::parse(&r.kind).expect("DB の CHECK と同じ値"),
            topic_id: r.topic_id,
            statement_id: r.statement_id,
            body: r.body,
            agree: r.agree as u32,
            disagree: r.disagree as u32,
            seen: r.seen as u32,
            groups: serde_json::from_str(&r.groups).unwrap_or_default(),
        })
        .collect())
}

/// 提言の一覧の1件
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub published_at: Option<String>,
    /// 合意点の数
    pub consensus: i64,
    /// 意見が分かれた点の数
    pub divided: i64,
}

/// 自治体の提言（`status` で絞る。公開済みは公開の新しい順、ほかは作った新しい順）
pub async fn list(
    pool: &SqlitePool,
    municipality_id: &str,
    status: Publication,
) -> Result<Vec<Summary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT r.id, r.title, r.status, r.created_at, r.published_at,
                (SELECT count(*) FROM recommendation_items i
                  WHERE i.recommendation_id = r.id AND i.kind = 'consensus') AS consensus,
                (SELECT count(*) FROM recommendation_items i
                  WHERE i.recommendation_id = r.id AND i.kind = 'divided') AS divided
         FROM recommendations r
         WHERE r.municipality_id = $1 AND r.status = $2
         ORDER BY COALESCE(r.published_at, r.created_at) DESC, r.id DESC",
    )
    .bind(municipality_id)
    .bind(status.as_str())
    .fetch_all(pool)
    .await
}

/// 元のトピック
#[derive(Debug, Clone, Serialize)]
pub struct TopicRef {
    pub id: String,
    pub title: String,
    /// 分析に含まれた人数（作ったとき）
    pub participants: i64,
    /// 関わり方ごとの人数（隠した区分は null）
    pub residency: BTreeMap<String, Option<i64>>,
}

/// ほかの提言・トピックへのリンク
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Link {
    pub id: String,
    pub title: String,
}

/// 提言の全体
#[derive(Debug, Clone, Serialize)]
pub struct Detail {
    pub id: String,
    pub municipality_id: String,
    pub title: String,
    pub preface: String,
    pub status: String,
    pub consensus_threshold: f64,
    pub divided_threshold: f64,
    pub created_at: String,
    pub published_at: Option<String>,
    pub topics: Vec<TopicRef>,
    /// 子の提言
    pub children: Vec<Link>,
    /// この提言を子に持つ公開済みの提言
    pub parents: Vec<Link>,
    /// この提言から作った第2回のトピック（公開済みのもの）
    pub second_round: Vec<Link>,
    pub items: Vec<Item>,
}

#[derive(sqlx::FromRow)]
struct Head {
    id: String,
    municipality_id: String,
    title: String,
    preface: String,
    status: String,
    consensus_threshold: f64,
    divided_threshold: f64,
    created_at: String,
    published_at: Option<String>,
}

pub async fn show(pool: &SqlitePool, id: &str) -> Result<Option<Detail>, sqlx::Error> {
    let Some(h): Option<Head> = sqlx::query_as(
        "SELECT id, municipality_id, title, preface, status, consensus_threshold, divided_threshold,
                created_at, published_at
         FROM recommendations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let topics: Vec<(String, String, i64, String)> = sqlx::query_as(
        "SELECT t.id, t.title, rt.participants, rt.residency
         FROM recommendation_topics rt JOIN topics t ON t.id = rt.topic_id
         WHERE rt.recommendation_id = $1 ORDER BY rt.position",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let children: Vec<Link> = sqlx::query_as(
        "SELECT r.id, r.title FROM recommendation_children c JOIN recommendations r ON r.id = c.child_id
         WHERE c.parent_id = $1 ORDER BY c.position",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let parents: Vec<Link> = sqlx::query_as(
        "SELECT r.id, r.title FROM recommendation_children c JOIN recommendations r ON r.id = c.parent_id
         WHERE c.child_id = $1 AND r.status = 'published' ORDER BY r.published_at, r.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let second_round: Vec<Link> = sqlx::query_as(
        "SELECT id, title FROM topics
         WHERE recommendation_id = $1 AND status = 'published' ORDER BY published_at, id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(Some(Detail {
        items: items(pool, id).await?,
        id: h.id,
        municipality_id: h.municipality_id,
        title: h.title,
        preface: h.preface,
        status: h.status,
        consensus_threshold: h.consensus_threshold,
        divided_threshold: h.divided_threshold,
        created_at: h.created_at,
        published_at: h.published_at,
        topics: topics
            .into_iter()
            .map(|(id, title, participants, residency)| TopicRef {
                id,
                title,
                participants,
                residency: serde_json::from_str(&residency).unwrap_or_default(),
            })
            .collect(),
        children,
        parents,
        second_round,
    }))
}

/// トピックをまとめた公開済みの提言（トピックの画面のリンク）
pub async fn published_for_topic(
    pool: &SqlitePool,
    topic_id: &str,
) -> Result<Vec<Link>, sqlx::Error> {
    sqlx::query_as(
        "SELECT r.id, r.title FROM recommendation_topics rt JOIN recommendations r ON r.id = rt.recommendation_id
         WHERE rt.topic_id = $1 AND r.status = 'published' ORDER BY r.published_at, r.id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("下書きの提言が見つかりません（公開した提言は変えません）")]
    NotDraft,
    #[error("見出しは{MAX}文字までです", MAX = crate::process::recommendation::MAX_TITLE_CHARS)]
    TitleTooLong,
    #[error("前書きは{MAX}文字までです", MAX = crate::process::recommendation::MAX_PREFACE_CHARS)]
    PrefaceTooLong,
}

/// 下書きの見出し・前書きを直す（`None` は変えない）
pub async fn edit(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    preface: Option<&str>,
) -> Result<Result<(), EditError>, sqlx::Error> {
    use crate::process::recommendation::{MAX_PREFACE_CHARS, MAX_TITLE_CHARS as MAX_HEAD};
    if title.is_some_and(|t| t.trim().chars().count() > MAX_HEAD) {
        return Ok(Err(EditError::TitleTooLong));
    }
    if preface.is_some_and(|p| p.trim().chars().count() > MAX_PREFACE_CHARS) {
        return Ok(Err(EditError::PrefaceTooLong));
    }
    let done = sqlx::query(
        "UPDATE recommendations SET title = COALESCE($1, title), preface = COALESCE($2, preface)
         WHERE id = $3 AND status = 'draft'",
    )
    .bind(title.map(str::trim))
    .bind(preface.map(str::trim))
    .bind(id)
    .execute(pool)
    .await?;
    Ok(if done.rows_affected() == 0 {
        Err(EditError::NotDraft)
    } else {
        Ok(())
    })
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum StatusError {
    #[error("提言が見つかりません")]
    NotFound,
    #[error("見出しを書いてから公開してください")]
    NoTitle,
    #[error("合意点も意見が分かれた点も無いので、公開できません")]
    NoItems,
    #[error("公開できるのは下書きだけ、取り下げられるのは公開済みだけです")]
    WrongState,
}

/// 公開する（下書きだけ）・取り下げる（公開済みだけ）
pub async fn set_status(
    pool: &SqlitePool,
    id: &str,
    status: Publication,
) -> Result<Result<(), StatusError>, sqlx::Error> {
    let row: Option<(String, String, i64)> = sqlx::query_as(
        "SELECT status, title, (SELECT count(*) FROM recommendation_items WHERE recommendation_id = $1)
         FROM recommendations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some((current, title, n_items)) = row else {
        return Ok(Err(StatusError::NotFound));
    };
    let ok = match status {
        Publication::Published => current == "draft",
        Publication::Archived => current == "published",
        Publication::Draft => false,
    };
    if !ok {
        return Ok(Err(StatusError::WrongState));
    }
    if status == Publication::Published {
        if title.trim().is_empty() {
            return Ok(Err(StatusError::NoTitle));
        }
        if n_items == 0 {
            return Ok(Err(StatusError::NoItems));
        }
    }
    sqlx::query(
        "UPDATE recommendations
         SET status = $1, published_at = CASE WHEN $1 = 'published' THEN $2 ELSE published_at END
         WHERE id = $3",
    )
    .bind(status.as_str())
    .bind(now_iso())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(Ok(()))
}

/// 下書きを消す（公開した提言は消さない。取り下げる）。消したら true
pub async fn remove_draft(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM recommendations WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    if status.as_deref() != Some("draft") {
        return Ok(false);
    }
    for sql in [
        "DELETE FROM recommendation_items WHERE recommendation_id = $1",
        "DELETE FROM recommendation_topics WHERE recommendation_id = $1",
        "DELETE FROM recommendation_children WHERE parent_id = $1",
        "DELETE FROM recommendations WHERE id = $1",
    ] {
        sqlx::query(sql).bind(id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(true)
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SecondRoundError {
    #[error("公開済みの提言が見つかりません")]
    NotPublished,
}

/// 公開した提言から、第2回のトピックの下書きを作る。合意点と意見が分かれた点の原文を最初の意見にする
/// （同じ文は1件に。多すぎるものは運営者が下書きで減らす）。戻り値はトピックの ID
pub async fn second_round(
    pool: &SqlitePool,
    id: &str,
) -> Result<Result<String, SecondRoundError>, sqlx::Error> {
    let Some(detail) = show(pool, id).await? else {
        return Ok(Err(SecondRoundError::NotPublished));
    };
    if detail.status != "published" {
        return Ok(Err(SecondRoundError::NotPublished));
    }
    // 元のトピックの種類を受け継ぐ（混ざっていれば議会。どちらでもないものは無い）
    let item_topics: Vec<&str> = detail.items.iter().map(|i| i.topic_id.as_str()).collect();
    let mut kinds = Vec::new();
    for topic_id in &item_topics {
        let kind: String = sqlx::query_scalar("SELECT source_type FROM topics WHERE id = $1")
            .bind(topic_id)
            .fetch_one(pool)
            .await?;
        kinds.push(kind);
    }
    let source_type = if !kinds.is_empty() && kinds.iter().all(|k| k == "citizen") {
        "citizen"
    } else {
        "assembly"
    };
    let title: String = detail.title.chars().take(MAX_TITLE_CHARS).collect();
    let description: String = format!(
        "提言「{}」の合意点と意見が分かれた点を、最初の意見にしました。あらためて投票してください。",
        detail.title
    )
    .chars()
    .take(MAX_DESCRIPTION_CHARS)
    .collect();

    let topic_id = new_id();
    let now = now_iso();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO topics
            (id, municipality_id, title, description, source_type, status, recommendation_id, created_at)
         VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7)",
    )
    .bind(&topic_id)
    .bind(&detail.municipality_id)
    .bind(&title)
    .bind(&description)
    .bind(source_type)
    .bind(id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    let mut seen = std::collections::HashSet::new();
    for item in &detail.items {
        if !seen.insert(item.body.as_str()) {
            continue;
        }
        sqlx::query(
            "INSERT INTO statements (id, topic_id, body, status, created_at, is_seed)
             VALUES ($1, $2, $3, 'approved', $4, TRUE)",
        )
        .bind(new_id())
        .bind(&topic_id)
        .bind(&item.body)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Ok(topic_id))
}
