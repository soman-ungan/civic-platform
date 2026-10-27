-- SPDX-License-Identifier: AGPL-3.0-only
-- 分野タグの共通語彙（全自治体共通のマスタ）。自動タグ付けはこの中からだけ選ぶ
-- 人物（首長・議員など）はタグにしない（docs/plans/2026-10-27-phase4-ai.md）
-- 名前の変更が既存の DB にも反映されるよう、id が同じなら名前を更新する
INSERT INTO tags (id, kind, name) VALUES
    ('field-kosodate',     'field', '子育て'),
    ('field-kyouiku',      'field', '教育'),
    ('field-fukushi',      'field', '福祉'),
    ('field-iryou',        'field', '医療・健康'),
    ('field-bousai',       'field', '防災・安全'),
    ('field-koutsu',       'field', '交通'),
    ('field-kankyou',      'field', '環境'),
    ('field-nourin',       'field', '農林業'),
    ('field-kankou',       'field', '観光'),
    ('field-sangyou',      'field', '産業・雇用'),
    ('field-zaisei',       'field', '財政'),
    ('field-machidukuri',  'field', 'まちづくり'),
    ('field-bunka',        'field', '文化・スポーツ'),
    ('field-gyousei',      'field', '行政運営')
ON CONFLICT (id) DO UPDATE SET name = excluded.name;
