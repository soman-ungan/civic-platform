-- SPDX-License-Identifier: AGPL-3.0-only
-- 自動タグ付け（docs/plans/2026-10-27-phase4-ai.md Step 3）
-- AI による付け直しは、運営者が付けたタグ（admin）を上書きしない
ALTER TABLE topic_tags ADD COLUMN assigned_by TEXT NOT NULL DEFAULT 'admin'
    CHECK (assigned_by IN ('ai', 'admin'));
-- AI がタグ付けを済ませた日時（候補に当てはまるタグが無かった場合も入れ、繰り返し呼ばないようにする）
ALTER TABLE topics ADD COLUMN tagged_at TEXT;
