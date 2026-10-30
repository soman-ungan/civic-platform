-- SPDX-License-Identifier: AGPL-3.0-only
-- 下書きの自動公開（ADR 0005「下書きを少しずつ公開する」。docs/plans/2026-10-30-timeline-flow.md）。
-- queued_at は公開待ちに入れた日時（NULL は入っていない）。status は draft のまま（下書きは外に出さない）。
-- 待ちの古い順に、publish_slots の時刻ごとに1件ずつ公開する
ALTER TABLE topics ADD COLUMN queued_at TEXT;

-- 公開の時刻（自治体ごと。1つの時刻で1日1件）。運営者が管理画面で足す・消す・変える
CREATE TABLE publish_slots (
    id              TEXT PRIMARY KEY,
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    -- 日本時間の HH:MM
    time_of_day     TEXT NOT NULL,
    -- 最後に動いた日（日本時間の YYYY-MM-DD）。同じ日に二度公開しないため
    last_run_on     TEXT,
    created_at      TEXT NOT NULL,
    UNIQUE (municipality_id, time_of_day)
);
