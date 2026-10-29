-- SPDX-License-Identifier: AGPL-3.0-only
-- トピックの期限（ADR 0005）。closes_at を過ぎたら締め切り（投票と投稿を止める。結果は読める）。
-- 公開したときに published_at と closes_at（90日後）を入れる。NULL は期限なし（この列を足す前に公開したもの）
ALTER TABLE topics ADD COLUMN published_at TEXT;
ALTER TABLE topics ADD COLUMN closes_at TEXT;
