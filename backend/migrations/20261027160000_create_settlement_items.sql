-- SPDX-License-Identifier: AGPL-3.0-only
-- 決算カード（総務省）から読み取った数値（docs/plans/2026-10-27-phase5b-settlement-cards.md）。
-- 原本（Excel）は持たず、数値と出典（documents）だけを残す

CREATE TABLE settlement_items (
    municipality_id TEXT NOT NULL REFERENCES municipalities (id),
    fiscal_year     INTEGER NOT NULL,
    -- revenue: 歳入 / expenditure_purpose: 目的別歳出 / expenditure_nature: 性質別歳出 / indicator: 財政指標など
    category        TEXT NOT NULL
                    CHECK (category IN ('revenue', 'expenditure_purpose', 'expenditure_nature', 'indicator')),
    -- 原表の表記から空白を除いたもの（例: '民生費'）。年度をまたいで同じ費目を並べるのに使う
    item_key        TEXT NOT NULL,
    -- 原表での並び順
    position        INTEGER NOT NULL,
    -- thousand_yen: 千円 / milli: 値の1000倍（比率・指数。浮動小数点の誤差を避けるため整数にする）。
    -- 原表が「-」（該当なし）なら NULL
    value           BIGINT,
    unit            TEXT NOT NULL CHECK (unit IN ('thousand_yen', 'milli')),
    document_id     TEXT NOT NULL REFERENCES documents (id),
    PRIMARY KEY (municipality_id, fiscal_year, category, item_key)
);

-- 費目と分野タグの対応（全自治体で共通。data/seeds/025_settlement_item_tags.sql）。
-- トピックの詳細に、その分野に関係する費目の数字を出すのに使う。対応はおおまか
CREATE TABLE settlement_item_tags (
    category TEXT NOT NULL,
    item_key TEXT NOT NULL,
    tag_id   TEXT NOT NULL REFERENCES tags (id),
    PRIMARY KEY (category, item_key, tag_id)
);
