-- SPDX-License-Identifier: AGPL-3.0-only
-- 当初予算の概要（自治体の PDF）から取り出した数字と事業（docs/plans/2026-10-27-phase5c-budget.md）。
-- 原本は持たず、数字・事業の名前・出典だけを残す。事業の説明の文章は持たない（市の著作物のため）

-- 年度ごとの予算。AI の抽出の結果なので、運営者が確かめてから公開する（draft → published）
CREATE TABLE budgets (
    id              TEXT PRIMARY KEY,
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    fiscal_year     INTEGER NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('draft', 'published', 'archived')),
    document_id     TEXT NOT NULL REFERENCES documents (id),
    -- 一般会計の歳出の合計（千円）
    total           BIGINT NOT NULL,
    created_at      TEXT NOT NULL,
    published_at    TEXT,
    UNIQUE (municipality_id, fiscal_year)
);

-- 区分ごとの額（千円）。費目のキーは決算カード（settlement_items）と同じ表記にそろえる（比べるため）
CREATE TABLE budget_items (
    budget_id TEXT NOT NULL REFERENCES budgets (id),
    -- revenue: 歳入 / expenditure_purpose: 目的別歳出 / expenditure_nature: 性質別歳出
    category  TEXT NOT NULL CHECK (category IN ('revenue', 'expenditure_purpose', 'expenditure_nature')),
    item_key  TEXT NOT NULL,
    position  INTEGER NOT NULL,
    value     BIGINT NOT NULL,
    PRIMARY KEY (budget_id, category, item_key)
);

-- 主な事業。名前と金額は、資料のそのページの本文にそのまま含まれることを確かめたものだけ
CREATE TABLE budget_projects (
    id         TEXT PRIMARY KEY,
    budget_id  TEXT NOT NULL REFERENCES budgets (id),
    position   INTEGER NOT NULL,
    name       TEXT NOT NULL,
    -- 千円
    amount     BIGINT NOT NULL,
    -- new: 新規 / expanded: 拡充・増額（資料に印があるときだけ）
    flag       TEXT CHECK (flag IN ('new', 'expanded')),
    -- その自治体での政策・分野の名前（総合計画など）。無い自治体もある
    plan_label TEXT,
    -- PDF の何ページ目か（1始まり）。原本へのリンクに使う
    page       INTEGER NOT NULL
);

CREATE INDEX idx_budget_projects_budget ON budget_projects (budget_id);

CREATE TABLE budget_project_tags (
    project_id TEXT NOT NULL REFERENCES budget_projects (id),
    tag_id     TEXT NOT NULL REFERENCES tags (id),
    PRIMARY KEY (project_id, tag_id)
);
