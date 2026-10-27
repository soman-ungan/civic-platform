-- SPDX-License-Identifier: AGPL-3.0-only

-- 栃木県大田原市（パイロット自治体）
-- 自治体を追加するときは、このファイルを参考に 1 自治体 1 ファイルで追加する。
-- 団体コード: 全国地方公共団体コード 6 桁（5 桁コード 09210 + 検査数字 0）

INSERT INTO municipalities (id, prefecture_code, name, name_kana, slug, official_url, is_active, created_at) VALUES
    ('092100', '09', '大田原市', 'おおたわらし', 'otawara', 'https://www.city.ohtawara.tochigi.jp/', TRUE, '2026-10-26T00:00:00Z')
ON CONFLICT DO NOTHING;

-- 地区: 話題の対象地域を示すためのもの（人を地区で分けない。ADR 0002）
--   旧黒羽町・旧湯津上村は 2005 年の合併前の旧町村の単位
--   旧大田原市は、暮らしの実感に合わせて昭和の合併前の町村の単位に分ける
--   （大田原 = 旧大田原町。金田・親園・野崎・佐久山。2026-10-27 ユーザーと決定）
-- 地区 ID は公的コードがないため「団体コード-ローマ字」の固定値とする
INSERT INTO districts (id, municipality_id, name, name_kana) VALUES
    ('092100-otawara',  '092100', '大田原', 'おおたわら'),
    ('092100-kaneda',   '092100', '金田',   'かねだ'),
    ('092100-chikasono','092100', '親園',   'ちかその'),
    ('092100-nozaki',   '092100', '野崎',   'のざき'),
    ('092100-sakuyama', '092100', '佐久山', 'さくやま'),
    ('092100-kurobane', '092100', '黒羽',   'くろばね'),
    ('092100-yuzukami', '092100', '湯津上', 'ゆづかみ')
ON CONFLICT DO NOTHING;

-- 収集先（docs/design/collection.md）。自治体ごとの違いは config に閉じ込める
--   決算カード（総務省）: 全市町村で同じ設定。都道府県と団体コードは municipalities から引く。
--     人口（municipality_stats）も決算カードの住民基本台帳人口から入る
--     規約: 総務省ホームページの「著作権について」で、公共データ利用規約（第1.0版）に準拠。出典の記載が条件
--     robots.txt: ia_archiver 以外は制限なし（2026-10-27 確認）
INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url, config, is_enabled) VALUES
    ('092100-settlement-card', '092100', 'settlement_card', 'soumu_settlement_card',
     'https://www.soumu.go.jp/iken/zaisei/card.html',
     '{"terms_url": "https://www.soumu.go.jp/menu_kyotsuu/policy/tyosaku.html", "terms_checked_at": "2026-10-27", "min_interval_secs": 5, "from_fiscal_year": 2015}',
     TRUE)
ON CONFLICT DO NOTHING;
