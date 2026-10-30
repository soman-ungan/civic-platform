// SPDX-License-Identifier: AGPL-3.0-only

//! 下書きの自動公開の公開待ちと公開の時刻（CLI の list / add / remove queue・slot、管理画面）。
//! 公開待ちに入れても `status` は draft のまま（下書きは外に出さない）。
//! いつ公開するかの決まりは `process::publishing`（docs/plans/2026-10-30-timeline-flow.md）

use sqlx::SqlitePool;

use crate::shared::util::{new_id, now_iso};

/// 公開待ちの1件
#[derive(Debug, sqlx::FromRow)]
pub struct QueuedTopic {
    pub id: String,
    pub title: String,
    /// assembly / citizen
    pub source_type: String,
    pub queued_at: String,
}

/// 公開待ちに入れる（下書きだけ）。もう入っていれば順番はそのまま。下書きが見つからなければ false。
/// `municipality_id` が無ければ自治体を問わない（CLI）
pub async fn enqueue(
    pool: &SqlitePool,
    municipality_id: Option<&str>,
    topic_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "UPDATE topics SET queued_at = COALESCE(queued_at, $1)
         WHERE id = $2 AND (CAST($3 AS TEXT) IS NULL OR municipality_id = $3) AND status = 'draft'",
    )
    .bind(now_iso())
    .bind(topic_id)
    .bind(municipality_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
}

/// 公開待ちから外す（下書きのまま残る）。下書きが見つからなければ false
pub async fn dequeue(
    pool: &SqlitePool,
    municipality_id: Option<&str>,
    topic_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "UPDATE topics SET queued_at = NULL
         WHERE id = $1 AND (CAST($2 AS TEXT) IS NULL OR municipality_id = $2) AND status = 'draft'",
    )
    .bind(topic_id)
    .bind(municipality_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
}

/// 自治体の公開待ち（公開する順。先に入れたものから）
pub async fn queue(
    pool: &SqlitePool,
    municipality_id: &str,
) -> Result<Vec<QueuedTopic>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, source_type, queued_at FROM topics
         WHERE municipality_id = $1 AND status = 'draft' AND queued_at IS NOT NULL
         ORDER BY queued_at, id",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await
}

/// 公開の時刻（1つで1日1件）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Slot {
    pub id: String,
    pub municipality_id: String,
    /// 日本時間の HH:MM
    pub time_of_day: String,
    /// 最後に動いた日（日本時間の YYYY-MM-DD）
    pub last_run_on: Option<String>,
}

/// 自治体の公開の時刻（早い順）。`municipality_id` が無ければすべての自治体
pub async fn slots(
    pool: &SqlitePool,
    municipality_id: Option<&str>,
) -> Result<Vec<Slot>, sqlx::Error> {
    // PostgreSQL では型の決まらないパラメーターを IS NULL にできないため CAST する
    sqlx::query_as(
        "SELECT id, municipality_id, time_of_day, last_run_on FROM publish_slots
         WHERE CAST($1 AS TEXT) IS NULL OR municipality_id = $1
         ORDER BY municipality_id, time_of_day",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await
}

/// 時刻の手直しの失敗
#[derive(Debug, thiserror::Error)]
pub enum SlotError {
    #[error("見つかりません: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// 時刻を確かめて、`HH:MM`（ゼロ埋め）にそろえる。文字列のまま比べるため
pub fn normalize_time(input: &str) -> Result<String, SlotError> {
    let invalid = || SlotError::Invalid(format!("時刻は 07:30 のように書いてください: {input}"));
    let (h, m) = input.trim().split_once(':').ok_or_else(invalid)?;
    let h: u32 = h.parse().map_err(|_| invalid())?;
    let m: u32 = m.parse().map_err(|_| invalid())?;
    if h > 23 || m > 59 {
        return Err(invalid());
    }
    Ok(format!("{h:02}:{m:02}"))
}

/// 時刻を足す。`last_run_on` は、今日の分をもう過ぎたなら今日（足したとたんに公開しないように）
pub async fn add_slot(
    pool: &SqlitePool,
    municipality_id: &str,
    time_of_day: &str,
    last_run_on: Option<&str>,
) -> Result<String, SlotError> {
    let id = new_id();
    let result = sqlx::query(
        "INSERT INTO publish_slots (id, municipality_id, time_of_day, last_run_on, created_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (municipality_id, time_of_day) DO NOTHING",
    )
    .bind(&id)
    .bind(municipality_id)
    .bind(time_of_day)
    .bind(last_run_on)
    .bind(now_iso())
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(SlotError::Invalid(format!(
            "{time_of_day} はもう登録されています"
        )));
    }
    Ok(id)
}

/// 時刻を変える（`last_run_on` は `add_slot` と同じ決まり）
pub async fn update_slot(
    pool: &SqlitePool,
    municipality_id: &str,
    slot_id: &str,
    time_of_day: &str,
    last_run_on: Option<&str>,
) -> Result<(), SlotError> {
    let taken: Option<String> = sqlx::query_scalar(
        "SELECT id FROM publish_slots WHERE municipality_id = $1 AND time_of_day = $2 AND id <> $3",
    )
    .bind(municipality_id)
    .bind(time_of_day)
    .bind(slot_id)
    .fetch_optional(pool)
    .await?;
    if taken.is_some() {
        return Err(SlotError::Invalid(format!(
            "{time_of_day} はもう登録されています"
        )));
    }
    let result = sqlx::query(
        "UPDATE publish_slots SET time_of_day = $1, last_run_on = $2
         WHERE id = $3 AND municipality_id = $4",
    )
    .bind(time_of_day)
    .bind(last_run_on)
    .bind(slot_id)
    .bind(municipality_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(SlotError::NotFound(slot_id.to_owned()));
    }
    Ok(())
}

/// 時刻を消す。`municipality_id` が無ければ自治体を問わない（CLI）
pub async fn remove_slot(
    pool: &SqlitePool,
    municipality_id: Option<&str>,
    slot_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "DELETE FROM publish_slots WHERE id = $1 AND (CAST($2 AS TEXT) IS NULL OR municipality_id = $2)",
    )
    .bind(slot_id)
    .bind(municipality_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
}

/// その日の分を動いたことにする。ほかで先に動いていたら false（同じ日に二度公開しない）
pub async fn claim_slot(
    pool: &SqlitePool,
    slot_id: &str,
    today: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "UPDATE publish_slots SET last_run_on = $1
         WHERE id = $2 AND (last_run_on IS NULL OR last_run_on < $1)",
    )
    .bind(today)
    .bind(slot_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
}
