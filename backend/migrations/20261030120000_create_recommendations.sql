-- SPDX-License-Identifier: AGPL-3.0-only
-- 提言（ADR 0005「トピックの統合は提言と第2回のトピックの順に作る」。docs/plans/2026-10-30-opinion-synthesis.md）。
-- 締め切ったトピックの合意点と意見が分かれた点を、原文と数字のまま集める。中身はプログラムだけで決める。
-- 作ったときの原文と数字を写して残す（あとで分析が変わっても、公開した提言は変わらない）
CREATE TABLE recommendations (
    id                  TEXT PRIMARY KEY,
    municipality_id     TEXT NOT NULL REFERENCES municipalities (id),
    -- 見出しと前書き（AI が下書きし、運営者が確かめる。下書きの間は空でもよい）
    title               TEXT NOT NULL DEFAULT '' CHECK (length(title) <= 60),
    preface             TEXT NOT NULL DEFAULT '' CHECK (length(preface) <= 400),
    status              TEXT NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft', 'published', 'archived')),
    -- 作ったときの基準（合意点: 全グループで (A+1)/(S+2) ≥ これ、意見が分かれた点: あるグループで賛成・別のグループで反対が ≥ これ）
    consensus_threshold REAL NOT NULL,
    divided_threshold   REAL NOT NULL,
    created_at          TEXT NOT NULL,
    published_at        TEXT
);

CREATE INDEX idx_recommendations_municipality ON recommendations (municipality_id, status);

-- 直接の元のトピック（締め切ったもの）。participants は分析に含まれた人数、
-- residency は関わり方ごとの人数（二次秘匿の済んだ JSON。隠した区分は null。ADR 0002）
CREATE TABLE recommendation_topics (
    recommendation_id TEXT NOT NULL REFERENCES recommendations (id),
    topic_id          TEXT NOT NULL REFERENCES topics (id),
    participants      INTEGER NOT NULL,
    residency         TEXT NOT NULL,
    position          INTEGER NOT NULL,
    PRIMARY KEY (recommendation_id, topic_id)
);

-- 子の提言（公開済みのものだけ。作るときに決め、あとから変えないので循環しない）
CREATE TABLE recommendation_children (
    parent_id TEXT NOT NULL REFERENCES recommendations (id),
    child_id  TEXT NOT NULL REFERENCES recommendations (id),
    position  INTEGER NOT NULL,
    PRIMARY KEY (parent_id, child_id),
    CHECK (parent_id <> child_id)
);

-- 項目（合意点・意見が分かれた点）。子の提言の項目も、原文と数字をたどって写す（子の見出し・前書きは使わない）。
-- agree / disagree / seen は分析に含まれた人の票の合計、groups はグループごとの票（JSON）
CREATE TABLE recommendation_items (
    id                TEXT PRIMARY KEY,
    recommendation_id TEXT NOT NULL REFERENCES recommendations (id),
    kind              TEXT NOT NULL CHECK (kind IN ('consensus', 'divided')),
    topic_id          TEXT NOT NULL REFERENCES topics (id),
    statement_id      TEXT NOT NULL REFERENCES statements (id),
    body              TEXT NOT NULL,
    agree             INTEGER NOT NULL,
    disagree          INTEGER NOT NULL,
    seen              INTEGER NOT NULL,
    groups            TEXT NOT NULL,
    position          INTEGER NOT NULL,
    UNIQUE (recommendation_id, statement_id)
);

CREATE INDEX idx_recommendation_items_recommendation ON recommendation_items (recommendation_id, position);

-- 第2回のトピック: 提言の中身を最初の意見にした新しいトピックの、元の提言
ALTER TABLE topics ADD COLUMN recommendation_id TEXT REFERENCES recommendations (id);

-- 提言の見出し・前書きの下書きを AI に頼んだ使用量を、ほかの用途と分けて記録する（purpose に 'recommendation' を足す）。
-- CHECK を変えるため作り直す（参照しているテーブルは無い）
CREATE TABLE llm_usage_new (
    id                          TEXT PRIMARY KEY,
    purpose                     TEXT NOT NULL
                                CHECK (purpose IN ('moderation', 'tagging', 'eval', 'extraction', 'recommendation')),
    model                       TEXT NOT NULL,
    input_tokens                INTEGER NOT NULL,
    output_tokens               INTEGER NOT NULL,
    cache_creation_input_tokens INTEGER NOT NULL,
    cache_read_input_tokens     INTEGER NOT NULL,
    cost_micro_usd              INTEGER NOT NULL,
    outcome                     TEXT NOT NULL
                                CHECK (outcome IN ('ok', 'refusal', 'max_tokens', 'invalid')),
    created_at                  TEXT NOT NULL
);
INSERT INTO llm_usage_new SELECT * FROM llm_usage;
DROP TABLE llm_usage;
ALTER TABLE llm_usage_new RENAME TO llm_usage;
CREATE INDEX idx_llm_usage_created ON llm_usage (created_at);
