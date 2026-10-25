-- 栃木県大田原市（パイロット自治体）
-- 自治体を追加するときは、このファイルを参考に 1 自治体 1 ファイルで追加する。
-- 団体コード: 全国地方公共団体コード 6 桁（5 桁コード 09210 + 検査数字 0）

INSERT INTO municipalities (id, prefecture_code, name, name_kana, slug, official_url, is_active, created_at) VALUES
    ('092100', '09', '大田原市', 'おおたわらし', 'otawara', 'https://www.city.ohtawara.tochigi.jp/', TRUE, '2026-10-26T00:00:00Z')
ON CONFLICT DO NOTHING;

-- 地区: 2005 年の合併前の旧市町村（旧大田原市・旧黒羽町・旧湯津上村）
-- 地区 ID は公的コードがないため「団体コード-ローマ字」の固定値とする
INSERT INTO districts (id, municipality_id, name, name_kana) VALUES
    ('092100-otawara',  '092100', '大田原', 'おおたわら'),
    ('092100-kurobane', '092100', '黒羽',   'くろばね'),
    ('092100-yuzukami', '092100', '湯津上', 'ゆづかみ')
ON CONFLICT DO NOTHING;

-- municipality_stats（人口・世帯数）は出典付きの値を確認してから登録する（フェーズ5）
