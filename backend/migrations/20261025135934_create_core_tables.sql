-- コア4テーブル（docs/grand-design.md §3）
-- SQLite / PostgreSQL の両方で動く構文のみを使う。
-- ID は TEXT（UUID v7）、日時はアプリ側で入れる ISO8601（UTC）文字列。

CREATE TABLE users (
    id             TEXT PRIMARY KEY,
    age_group      TEXT,
    gender         TEXT CHECK (gender IN ('male', 'female', 'other', 'prefer_not_to_say')),
    occupation     TEXT,
    -- resident: 大田原市民 / neighbor: 周辺住民 / external: 市外・関係人口
    residency_type TEXT CHECK (residency_type IN ('resident', 'neighbor', 'external')),
    created_at     TEXT NOT NULL
);

CREATE TABLE topics (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source_type TEXT NOT NULL CHECK (source_type IN ('assembly', 'citizen')),
    source_url  TEXT,
    -- JSON 配列の文字列（例: ["子育て", "黒羽地域"]）
    tags        TEXT NOT NULL DEFAULT '[]',
    created_at  TEXT NOT NULL
);

CREATE TABLE statements (
    id         TEXT PRIMARY KEY,
    topic_id   TEXT NOT NULL REFERENCES topics (id),
    -- NULL は議事録から AI が抽出した初期シード意見
    user_id    TEXT REFERENCES users (id),
    -- SQLite の length() は文字数。PostgreSQL では length() / char_length() が同じ意味
    body       TEXT NOT NULL CHECK (length(body) BETWEEN 1 AND 140),
    status     TEXT NOT NULL DEFAULT 'pending'
               CHECK (status IN ('pending', 'approved', 'rejected', 'flagged')),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_statements_topic_status ON statements (topic_id, status);

CREATE TABLE votes (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users (id),
    statement_id TEXT NOT NULL REFERENCES statements (id),
    -- 1: 賛成 / -1: 反対 / 0: パス
    vote_value   INTEGER NOT NULL CHECK (vote_value IN (-1, 0, 1)),
    created_at   TEXT NOT NULL,
    UNIQUE (user_id, statement_id)
);

CREATE INDEX idx_votes_statement ON votes (statement_id);
