-- SPDX-License-Identifier: AGPL-3.0-only
-- 大田原市の公人（首長・議員）の名簿。会議録の発言者との対応付けに使う
-- （docs/plans/2026-10-27-phase5a-minutes.md）
-- 選挙で選ばれた公人だけを登録する。職員（副市長・教育長・部長など）は登録しない（職名だけを示す）
-- 氏名は会議録の表記のとおり（髙・瀨などの字も変えない）
--
-- 【要確認】term_start（任期の開始日）は、正式な日付を確かめるまでの仮の値。
--   令和8年第1回定例会（2026-02-24〜03-13）の会議録で在職を確認したので、その初日を入れている。
--   選挙の日付などで確かめたら、正しい日付に直す
-- 出典: 大田原市議会 令和8年第1回定例会 会議録（第1号〜第6号）の出席議員・発言者
--   https://ssp.kaigiroku.net/tenant/ohtawara/MinuteView.html?council_id=608&schedule_id=2&is_search=false&view_years=2026
INSERT INTO public_officials (id, municipality_id, name, role, term_start, term_end, note) VALUES
    ('092100-mayor-souma-kenichi',     '092100', '相馬憲一',     'mayor',     '2026-02-24', NULL, '市長。任期の開始日は要確認'),
    ('092100-council-saitou-fujio',    '092100', '齋藤藤男',     'councilor', '2026-02-24', NULL, '1番。任期の開始日は要確認'),
    ('092100-council-iga-jun',         '092100', '伊賀純',       'councilor', '2026-02-24', NULL, '2番。任期の開始日は要確認'),
    ('092100-council-maeda-noritaka',  '092100', '前田則隆',     'councilor', '2026-02-24', NULL, '3番。任期の開始日は要確認'),
    ('092100-council-naitou-mikio',    '092100', '内藤幹夫',     'councilor', '2026-02-24', NULL, '5番。任期の開始日は要確認'),
    ('092100-council-kitahara-yuuko',  '092100', '北原裕子',     'councilor', '2026-02-24', NULL, '6番。任期の開始日は要確認'),
    ('092100-council-fujita-yoshiyuki','092100', '藤田善幸',     'councilor', '2026-02-24', NULL, '7番。任期の開始日は要確認'),
    ('092100-council-tsumori-nao',     '092100', '津守那音',     'councilor', '2026-02-24', NULL, '8番。任期の開始日は要確認'),
    ('092100-council-okano-tadashi',   '092100', '岡野忠',       'councilor', '2026-02-24', NULL, '9番。任期の開始日は要確認'),
    ('092100-council-ootsuka-masayoshi','092100','大塚正義',     'councilor', '2026-02-24', NULL, '10番。任期の開始日は要確認'),
    ('092100-council-akiyama-sachiko', '092100', '秋山幸子',     'councilor', '2026-02-24', NULL, '11番。任期の開始日は要確認'),
    ('092100-council-maeno-ryouzou',   '092100', '前野良三',     'councilor', '2026-02-24', NULL, '12番。任期の開始日は要確認'),
    ('092100-council-onodera-naotake', '092100', '小野寺尚武',   'councilor', '2026-02-24', NULL, '13番。任期の開始日は要確認'),
    ('092100-council-oomameuda-harumi','092100', '大豆生田春美', 'councilor', '2026-02-24', NULL, '14番。任期の開始日は要確認'),
    ('092100-council-nakagawa-masayuki','092100','中川雅之',     'councilor', '2026-02-24', NULL, '15番。任期の開始日は要確認'),
    ('092100-council-sakurai-junichirou','092100','櫻井潤一郎',  'councilor', '2026-02-24', NULL, '16番。任期の開始日は要確認'),
    ('092100-council-takase-shigetsugu','092100','髙瀨重嗣',     'councilor', '2026-02-24', NULL, '17番。任期の開始日は要確認'),
    ('092100-council-takita-ichirou',  '092100', '滝田一郎',     'councilor', '2026-02-24', NULL, '18番。任期の開始日は要確認'),
    ('092100-council-kikuchi-hisamitsu','092100','菊池久光',     'councilor', '2026-02-24', NULL, '19番。任期の開始日は要確認'),
    ('092100-council-kimishima-takaaki','092100','君島孝明',     'councilor', '2026-02-24', NULL, '20番、議長。任期の開始日は要確認'),
    -- 定例会の前に議員でなくなった人。発言者にはならないが、トピックの説明や意見に名前を入れないために登録する
    -- 【要確認】在職の期間。term_end は会議録に書かれた日（辞職願の提出日・報告の日）による仮の値
    ('092100-council-hikichi-tatsuo',  '092100', '引地達雄',     'councilor', '2026-02-20', '2026-02-20', '2026-02-20 に辞職願を提出（第1号の会議録で報告）。在職の期間は要確認'),
    ('092100-council-fukasawa-masao',  '092100', '深澤正夫',     'councilor', '2026-02-24', '2026-02-24', '第1号の会議録で死去を報告。在職の期間は要確認')
ON CONFLICT (id) DO UPDATE SET
    name = excluded.name, role = excluded.role, term_start = excluded.term_start,
    term_end = excluded.term_end, note = excluded.note;
