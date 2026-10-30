// SPDX-License-Identifier: AGPL-3.0-only

//! 下書きの自動公開（ADR 0005「下書きを少しずつ公開し、タイムラインに日々議題が上がるようにする」。
//! docs/plans/2026-10-30-timeline-flow.md）。
//! 自治体ごとの公開の時刻（`publish_slots`）が来るたびに、公開待ちの古い順に1件を公開する。
//! backend が止まっていて時刻を逃しても、その日のうちに動けば公開する（日をまたいだ分はまとめて出さない）。
//! 時刻は日本時間（このサービスは日本の自治体だけを扱う）

use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
use sqlx::SqlitePool;

use crate::process::progress;
use crate::records::publishing::{self, Slot, SlotError};
use crate::records::topics;
use crate::shared::status::Publication;

/// 日本時間（夏時間なし）
fn jst() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).expect("範囲内")
}

/// 日本時間の日付（YYYY-MM-DD）と時刻（HH:MM）
fn local(now: DateTime<Utc>) -> (String, String) {
    let t = now.with_timezone(&jst());
    (
        t.format("%Y-%m-%d").to_string(),
        t.format("%H:%M").to_string(),
    )
}

/// 時刻の枠が今動くか: 今日まだ動いておらず、時刻を過ぎた
fn due(slot: &Slot, today: &str, now_hm: &str) -> bool {
    slot.last_run_on.as_deref().is_none_or(|d| d < today) && slot.time_of_day.as_str() <= now_hm
}

/// 時刻の来た枠ごとに、その自治体の公開待ちの古い順に1件を公開する。公開したトピックの ID を返す。
/// 待ちが空なら、その枠は何もしない（その日の分は動いたことにする）
pub async fn run_due(pool: &SqlitePool, now: DateTime<Utc>) -> Result<Vec<String>, sqlx::Error> {
    let (today, now_hm) = local(now);
    let mut published = Vec::new();
    for slot in publishing::slots(pool, None).await? {
        if !due(&slot, &today, &now_hm) || !publishing::claim_slot(pool, &slot.id, &today).await? {
            continue;
        }
        let Some(next) = publishing::queue(pool, &slot.municipality_id)
            .await?
            .into_iter()
            .next()
        else {
            continue;
        };
        topics::set_status(
            pool,
            &next.id,
            Publication::Published,
            &progress::default_closes_at(),
        )
        .await?;
        published.push(next.id);
    }
    Ok(published)
}

/// 1分ごとに、時刻の来た枠の公開をする
pub fn spawn_worker(pool: SqlitePool) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            match run_due(&pool, Utc::now()).await {
                Ok(ids) if ids.is_empty() => {}
                Ok(ids) => tracing::info!(topics = ?ids, "scheduled topics published"),
                Err(err) => tracing::error!(error = %err, "publishing worker failed"),
            }
        }
    });
}

/// 時刻を足す・変えたときの `last_run_on`: 今日の時刻をもう過ぎていれば今日（足したとたんに公開しないように）
fn skip_today(time_of_day: &str, now: DateTime<Utc>) -> Option<String> {
    let (today, now_hm) = local(now);
    (time_of_day <= now_hm.as_str()).then_some(today)
}

/// 公開の時刻を足す（`HH:MM`）。今日の時刻を過ぎていれば、明日から動く
pub async fn add_slot(
    pool: &SqlitePool,
    municipality_id: &str,
    time_of_day: &str,
    now: DateTime<Utc>,
) -> Result<String, SlotError> {
    let time = publishing::normalize_time(time_of_day)?;
    let skip = skip_today(&time, now);
    publishing::add_slot(pool, municipality_id, &time, skip.as_deref()).await
}

/// 公開の時刻を変える。今日の時刻を過ぎていれば、明日から動く
pub async fn update_slot(
    pool: &SqlitePool,
    municipality_id: &str,
    slot_id: &str,
    time_of_day: &str,
    now: DateTime<Utc>,
) -> Result<(), SlotError> {
    let time = publishing::normalize_time(time_of_day)?;
    let skip = skip_today(&time, now);
    publishing::update_slot(pool, municipality_id, slot_id, &time, skip.as_deref()).await
}

/// 公開待ちのそれぞれを公開する予定の日時（ISO8601、UTC）。時刻が1つも無ければ空。
/// 今の時刻の決まりのまま進んだときの見込み（運営者が手で公開すれば、後ろが繰り上がる）
pub fn planned(slots: &[Slot], count: usize, now: DateTime<Utc>) -> Vec<String> {
    if slots.is_empty() {
        return Vec::new();
    }
    let (today, now_hm) = local(now);
    let today_date = NaiveDate::parse_from_str(&today, "%Y-%m-%d").expect("形は決まっている");
    let mut sorted: Vec<&Slot> = slots.iter().collect();
    sorted.sort_by(|a, b| a.time_of_day.cmp(&b.time_of_day));
    let mut out = Vec::with_capacity(count);
    let mut day = today_date;
    while out.len() < count {
        for slot in &sorted {
            if out.len() == count {
                break;
            }
            let at = if day != today_date {
                at_jst(day, &slot.time_of_day)
            } else if due(slot, &today, &now_hm) {
                // 時刻を過ぎて、まだ動いていない枠は、次のワーカーの番（1分以内）に動く
                now
            } else if slot.time_of_day.as_str() <= now_hm.as_str()
                || slot.last_run_on.as_deref() == Some(today.as_str())
            {
                // 今日の分はもう動いた
                continue;
            } else {
                at_jst(day, &slot.time_of_day)
            };
            out.push(at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        }
        day = day.succ_opt().expect("範囲内");
    }
    out
}

/// 日本時間の日付と HH:MM → UTC
fn at_jst(day: NaiveDate, hm: &str) -> DateTime<Utc> {
    let time = NaiveTime::parse_from_str(hm, "%H:%M").unwrap_or(NaiveTime::MIN);
    jst()
        .from_local_datetime(&day.and_time(time))
        .single()
        .expect("日本時間は一意")
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(time: &str, last: Option<&str>) -> Slot {
        Slot {
            id: time.into(),
            municipality_id: "m".into(),
            time_of_day: time.into(),
            last_run_on: last.map(Into::into),
        }
    }

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn planned_follows_the_slots_in_japan_time() {
        // 日本時間 2026-10-30 10:00。07:00 は今日もう動いた、12:00 はまだ
        let now = utc("2026-10-30T01:00:00Z");
        let slots = [
            slot("07:00", Some("2026-10-30")),
            slot("12:00", Some("2026-10-29")),
        ];
        assert_eq!(
            planned(&slots, 3, now),
            [
                "2026-10-30T03:00:00.000Z", // 今日 12:00
                "2026-10-30T22:00:00.000Z", // 明日 07:00
                "2026-10-31T03:00:00.000Z", // 明日 12:00
            ]
        );
        assert!(planned(&[], 3, now).is_empty());
    }

    #[test]
    fn planned_counts_a_missed_slot_as_now() {
        // 日本時間 10:00。07:00 をまだ動いていない（backend が止まっていた）
        let now = utc("2026-10-30T01:00:00Z");
        let slots = [slot("07:00", Some("2026-10-29"))];
        assert_eq!(
            planned(&slots, 2, now),
            ["2026-10-30T01:00:00.000Z", "2026-10-30T22:00:00.000Z"]
        );
    }
}
