-- SPDX-License-Identifier: AGPL-3.0-only
-- 最初の意見かどうか（docs/plans/2026-10-28-similar-topics.md）。
-- 議会の主張を、公開中のトピックにあとから足すことがある（最初の意見としては扱わない）ため、
-- 「投稿者がいない（user_id IS NULL）」だけでは最初の意見かを見分けられなくなる。
-- 今の分析の条件（すべての意見に投票）は変えない。フェーズ6で「最初の意見すべて＋一定の数」にするときに使う
ALTER TABLE statements ADD COLUMN is_seed BOOLEAN NOT NULL DEFAULT FALSE;

-- 今までの意見は、投稿者がいなければ最初の意見（会議録から作ったもの・運営者が足したもの）
UPDATE statements SET is_seed = TRUE WHERE user_id IS NULL;
