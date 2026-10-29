-- SPDX-License-Identifier: AGPL-3.0-only
-- 市民からのトピックの提案（ADR 0005。docs/plans/2026-10-29-phase6d-citizen-proposals.md）。
-- proposed_by は提案した人（source_type = 'citizen' だけに入る）。提案者の意見の上限と、自治体ごとの週1件に使う。
-- API のどこにも返さない（運営者の API でも。匿名性）
ALTER TABLE topics ADD COLUMN proposed_by TEXT REFERENCES users (id);

CREATE INDEX idx_topics_proposed_by ON topics (proposed_by, municipality_id, created_at);
