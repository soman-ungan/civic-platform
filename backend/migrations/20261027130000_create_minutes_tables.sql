-- SPDX-License-Identifier: AGPL-3.0-only
-- 会議録の取り込みと、トピックの出典（docs/plans/2026-10-27-phase5a-minutes.md）
-- 原本（本文）は保持しない。出典の情報と短い引用だけを残す

-- 取り込んだ資料（会議録・予算書・決算カード）。本文は持たない
CREATE TABLE documents (
    id                  TEXT PRIMARY KEY,
    municipality_id     TEXT NOT NULL REFERENCES municipalities (id),
    kind                TEXT NOT NULL CHECK (kind IN ('minutes', 'budget', 'settlement_card')),
    -- 会議名・資料名
    title               TEXT NOT NULL,
    -- 公開されているページの URL（手動の取り込みでは任意）
    url                 TEXT,
    -- 開催日（会議録）。YYYY-MM-DD
    held_on             TEXT,
    -- 本文のハッシュ。同じ資料を二度取り込まないため
    sha256              TEXT NOT NULL,
    char_count          INTEGER NOT NULL,
    collector_source_id TEXT REFERENCES collector_sources (id),
    imported_at         TEXT NOT NULL,
    UNIQUE (municipality_id, sha256)
);

-- 選挙で選ばれた公人（首長・議員）。発言者の対応付けに使う。職員は登録しない（職名だけを示す）
CREATE TABLE public_officials (
    id              TEXT PRIMARY KEY,
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    name            TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('mayor', 'councilor')),
    -- 任期（YYYY-MM-DD）。終了が NULL なら現職
    term_start      TEXT NOT NULL,
    term_end        TEXT,
    -- 出典など
    note            TEXT
);

CREATE INDEX idx_public_officials_municipality ON public_officials (municipality_id);

-- トピックの出典（会議録の発言からの引用）
CREATE TABLE topic_sources (
    id            TEXT PRIMARY KEY,
    topic_id      TEXT NOT NULL REFERENCES topics (id),
    document_id   TEXT NOT NULL REFERENCES documents (id),
    position      INTEGER NOT NULL,
    -- 会議録の中で何番目の発言か
    utterance_no  INTEGER NOT NULL,
    -- 表示する発言者。公人は「〇〇議員」「市長」など、職員は職名だけ（氏名は出さない）
    speaker_label TEXT NOT NULL,
    official_id   TEXT REFERENCES public_officials (id),
    -- 会議録の原文と一致する短い引用
    quote         TEXT NOT NULL CHECK (length(quote) BETWEEN 1 AND 200)
);

CREATE INDEX idx_topic_sources_topic ON topic_sources (topic_id);

-- AI が作ったトピックは下書きにし、運営者が確認してから公開する。既存のトピックは公開のまま
ALTER TABLE topics ADD COLUMN status TEXT NOT NULL DEFAULT 'published'
    CHECK (status IN ('draft', 'published', 'archived'));
