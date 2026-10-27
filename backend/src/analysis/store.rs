// SPDX-License-Identifier: AGPL-3.0-only

//! 分析結果の保存と、バックグラウンドでの再計算。

use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::conversation::{ConvResult, ConvState, Outcome, Vote, update};
use crate::util::now_iso;

/// データ不足のときに保存する内容
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insufficient {
    pub participants: usize,
    pub statements: usize,
}

/// 保存されている分析
#[derive(Debug, Clone)]
pub enum Stored {
    Ready {
        result: Box<ConvResult>,
        state: Box<ConvState>,
        computed_at: String,
    },
    Insufficient {
        detail: Insufficient,
        computed_at: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// 入力が前回と同じなので計算しなかった
    Unchanged,
    Computed,
}

#[derive(sqlx::FromRow)]
struct Row {
    input_fingerprint: String,
    status: String,
    result: String,
    state: String,
    computed_at: String,
}

async fn load_row(pool: &SqlitePool, topic_id: &str) -> Result<Option<Row>, sqlx::Error> {
    sqlx::query_as(
        "SELECT input_fingerprint, status, result, state, computed_at
         FROM topic_analyses WHERE topic_id = $1",
    )
    .bind(topic_id)
    .fetch_optional(pool)
    .await
}

/// 保存されている分析を読む（まだ無ければ None）
pub async fn load(pool: &SqlitePool, topic_id: &str) -> anyhow::Result<Option<Stored>> {
    let Some(row) = load_row(pool, topic_id).await? else {
        return Ok(None);
    };
    Ok(Some(match row.status.as_str() {
        "ready" => Stored::Ready {
            result: Box::new(serde_json::from_str(&row.result)?),
            state: Box::new(serde_json::from_str(&row.state)?),
            computed_at: row.computed_at,
        },
        _ => Stored::Insufficient {
            detail: serde_json::from_str(&row.result)?,
            computed_at: row.computed_at,
        },
    }))
}

/// 非公開の意見（承認済み以外）の id
async fn mod_out(pool: &SqlitePool, topic_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM statements WHERE topic_id = $1 AND status <> 'approved' ORDER BY id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await
}

/// 入力が変わったかを判定する値: 投票数・最新の投票時刻・非公開の意見
async fn fingerprint(
    pool: &SqlitePool,
    topic_id: &str,
    mod_out: &[String],
) -> Result<String, sqlx::Error> {
    let (count, latest): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*), max(v.created_at) FROM votes v
         JOIN statements s ON s.id = v.statement_id
         WHERE s.topic_id = $1",
    )
    .bind(topic_id)
    .fetch_one(pool)
    .await?;
    Ok(format!(
        "votes={count};latest={};mod_out={}",
        latest.unwrap_or_default(),
        mod_out.join(",")
    ))
}

/// トピックの分析を更新する。入力が前回と同じなら計算しない（`force` で強制）
pub async fn run_topic(
    pool: &SqlitePool,
    topic_id: &str,
    force: bool,
) -> anyhow::Result<RunOutcome> {
    let mod_out_ids = mod_out(pool, topic_id).await?;
    let fp = fingerprint(pool, topic_id, &mod_out_ids).await?;
    let previous = load_row(pool, topic_id).await?;
    if !force && previous.as_ref().is_some_and(|r| r.input_fingerprint == fp) {
        return Ok(RunOutcome::Unchanged);
    }
    let prev_state: ConvState = match &previous {
        Some(r) => serde_json::from_str(&r.state)?,
        None => ConvState::default(),
    };

    // 時刻の古い順。値は Polis の符号（賛成 = -1）に反転する
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT v.user_id, v.statement_id, v.vote_value FROM votes v
         JOIN statements s ON s.id = v.statement_id
         WHERE s.topic_id = $1
         ORDER BY v.created_at, v.id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let votes: Vec<Vote> = rows
        .into_iter()
        .map(|(pid, tid, value)| Vote {
            pid,
            tid,
            value: -(value as f64),
        })
        .collect();
    let mod_out: HashSet<String> = mod_out_ids.into_iter().collect();

    // 数理処理は CPU だけを使うので、ブロッキング用のスレッドで行う
    let outcome =
        tokio::task::spawn_blocking(move || update(&prev_state, &votes, &mod_out)).await?;
    let (status, result, state) = match outcome {
        Outcome::Ready { result, state } => (
            "ready",
            serde_json::to_string(&result)?,
            serde_json::to_string(&state)?,
        ),
        Outcome::Insufficient {
            participants,
            statements,
        } => (
            "insufficient",
            serde_json::to_string(&Insufficient {
                participants,
                statements,
            })?,
            // データ不足のときは前回の状態を残す
            match previous {
                Some(r) => r.state,
                None => serde_json::to_string(&ConvState::default())?,
            },
        ),
    };

    sqlx::query(
        "INSERT INTO topic_analyses (topic_id, input_fingerprint, status, result, state, computed_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (topic_id) DO UPDATE SET
            input_fingerprint = excluded.input_fingerprint,
            status = excluded.status,
            result = excluded.result,
            state = excluded.state,
            computed_at = excluded.computed_at",
    )
    .bind(topic_id)
    .bind(&fp)
    .bind(status)
    .bind(&result)
    .bind(&state)
    .bind(now_iso())
    .execute(pool)
    .await?;
    Ok(RunOutcome::Computed)
}

/// 公開済みの全トピックの分析を更新する。戻り値は計算したトピックの数
pub async fn run_all(pool: &SqlitePool, force: bool) -> anyhow::Result<usize> {
    let topic_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM topics WHERE status = 'published' ORDER BY id")
            .fetch_all(pool)
            .await?;
    let mut computed = 0;
    for topic_id in topic_ids {
        match run_topic(pool, &topic_id, force).await {
            Ok(RunOutcome::Computed) => computed += 1,
            Ok(RunOutcome::Unchanged) => {}
            Err(err) => tracing::error!(topic_id, error = %err, "analysis failed"),
        }
    }
    Ok(computed)
}

/// 一定間隔で、入力が変わったトピックの分析を更新し続ける（Polis の math worker に相当）
pub fn spawn_worker(pool: SqlitePool, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            match run_all(&pool, false).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(topics = n, "analysis updated"),
                Err(err) => tracing::error!(error = %err, "analysis worker failed"),
            }
        }
    });
}
