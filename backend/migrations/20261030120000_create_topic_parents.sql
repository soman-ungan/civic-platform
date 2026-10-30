-- SPDX-License-Identifier: AGPL-3.0-only
-- 市民のまとめから作ったトピックの、元のトピック（ADR 0005「市民のまとめ」。docs/plans/2026-10-30-topic-map.md）。
-- 市民がトピックマップで締め切ったトピックを選び、それを見ながら新しいトピックを提案すると、選んだトピックを親として残す。
-- 親子はトピックの画面からたどれる（公開済みのものだけ）。親を残すのは提案のときだけで、あとから変えない
CREATE TABLE topic_parents (
    topic_id  TEXT NOT NULL REFERENCES topics (id),
    parent_id TEXT NOT NULL REFERENCES topics (id),
    -- 選んだ順（0 が起点）
    position  INTEGER NOT NULL,
    PRIMARY KEY (topic_id, parent_id),
    CHECK (topic_id <> parent_id)
);

CREATE INDEX idx_topic_parents_parent ON topic_parents (parent_id);
