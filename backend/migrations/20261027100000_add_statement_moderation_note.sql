-- SPDX-License-Identifier: AGPL-3.0-only
-- AI のモデレーションが示した判定の理由（運営者の確認用。API では返さない）
ALTER TABLE statements ADD COLUMN moderation_note TEXT;
