-- コアスキーマ（docs/grand-design.md v2 §3、多自治体対応）
-- SQLite / PostgreSQL の両方で動く構文のみを使う。
-- ID は TEXT（UUID v7。行政区分マスタは公的コード）、日時はアプリ側で入れる ISO8601（UTC）文字列。
-- テーブルは外部キーの依存順に作成する。

-- ===== 行政区分 =====

CREATE TABLE prefectures (
    -- 都道府県コード 2桁（例: '09' = 栃木県）
    code TEXT PRIMARY KEY CHECK (length(code) = 2),
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE municipalities (
    -- 総務省「全国地方公共団体コード」6桁（検査数字込み）
    id              TEXT PRIMARY KEY CHECK (length(id) = 6),
    prefecture_code TEXT NOT NULL REFERENCES prefectures (code),
    name            TEXT NOT NULL,
    name_kana       TEXT NOT NULL,
    -- URL 用（例: '/otawara'）
    slug            TEXT NOT NULL UNIQUE,
    official_url    TEXT,
    -- 公開中の自治体のみ選択肢に表示する
    is_active       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TEXT NOT NULL
);

CREATE TABLE districts (
    id              TEXT PRIMARY KEY,
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    name            TEXT NOT NULL,
    name_kana       TEXT NOT NULL,
    UNIQUE (municipality_id, name),
    -- user_municipality_relations の複合外部キー用
    UNIQUE (id, municipality_id)
);

-- 「市民1人あたり年間〇〇円」換算に使う年度別統計
CREATE TABLE municipality_stats (
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    fiscal_year     INTEGER NOT NULL,
    population      INTEGER NOT NULL CHECK (population > 0),
    households      INTEGER CHECK (households > 0),
    source_url      TEXT NOT NULL,
    PRIMARY KEY (municipality_id, fiscal_year)
);

-- ===== ユーザー =====

CREATE TABLE users (
    id                   TEXT PRIMARY KEY,
    age_group            TEXT,
    gender               TEXT CHECK (gender IN ('male', 'female', 'other', 'prefer_not_to_say')),
    occupation           TEXT,
    -- 初期表示に使うホーム自治体
    home_municipality_id TEXT REFERENCES municipalities (id),
    created_at           TEXT NOT NULL
);

-- 自治体ごとのユーザーとの関係（居住地属性は自治体ごとに持つ）
CREATE TABLE user_municipality_relations (
    user_id         TEXT NOT NULL REFERENCES users (id),
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    -- resident: 住民 / neighbor: 周辺住民 / commuter: 通勤・通学 / related: 関係人口
    residency_type  TEXT NOT NULL CHECK (residency_type IN ('resident', 'neighbor', 'commuter', 'related')),
    district_id     TEXT,
    created_at      TEXT NOT NULL,
    PRIMARY KEY (user_id, municipality_id),
    -- 別の自治体の地区を指せないようにする（district_id が NULL なら検査しない）
    FOREIGN KEY (district_id, municipality_id) REFERENCES districts (id, municipality_id)
);

-- ===== Polis コア =====

CREATE TABLE topics (
    id              TEXT PRIMARY KEY,
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    title           TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    source_type     TEXT NOT NULL CHECK (source_type IN ('assembly', 'citizen')),
    source_url      TEXT,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_topics_municipality_created ON topics (municipality_id, created_at);

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

-- ===== タグ =====

-- field: 全自治体共通の分野タグ / district: districts に紐づく地区タグ
CREATE TABLE tags (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('field', 'district')),
    name        TEXT NOT NULL,
    district_id TEXT REFERENCES districts (id),
    CHECK (
        (kind = 'field' AND district_id IS NULL)
        OR (kind = 'district' AND district_id IS NOT NULL)
    )
);

-- NULL は UNIQUE 制約で重複とみなされないため、kind ごとの部分インデックスで一意にする
CREATE UNIQUE INDEX uq_tags_field_name ON tags (name) WHERE kind = 'field';
CREATE UNIQUE INDEX uq_tags_district ON tags (district_id) WHERE kind = 'district';

CREATE TABLE topic_tags (
    topic_id TEXT NOT NULL REFERENCES topics (id),
    tag_id   TEXT NOT NULL REFERENCES tags (id),
    PRIMARY KEY (topic_id, tag_id)
);

-- タグからトピックを引く（全国の #子育て など）
CREATE INDEX idx_topic_tags_tag ON topic_tags (tag_id);

-- ===== 収集 =====

CREATE TABLE collector_sources (
    id              TEXT PRIMARY KEY,
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    kind            TEXT NOT NULL CHECK (kind IN ('minutes', 'settlement_card', 'budget')),
    -- 使用する収集アダプタ名（例: 'generic_html'）
    adapter         TEXT NOT NULL,
    base_url        TEXT NOT NULL,
    -- アダプタ固有設定（JSON）
    config          TEXT NOT NULL DEFAULT '{}',
    is_enabled      BOOLEAN NOT NULL DEFAULT FALSE,
    last_crawled_at TEXT
);
