-- SPDX-License-Identifier: AGPL-3.0-only
-- 開発・動作確認用のサンプルデータ（本番には投入しない。`backend seed --dev` でのみ適用）
-- 内容はすべて架空。実際の議案・発言ではない。タイトルには必ず【サンプル】を付ける。
-- 前提: data/seeds/ の大田原市（092100）と地区、分野タグの語彙が登録済みであること。

-- 分野タグは data/seeds/020_field_tags.sql（全自治体共通の語彙）にある

-- 地区タグ（districts に紐づく）
INSERT INTO tags (id, kind, name, district_id) VALUES
    ('district-092100-kurobane', 'district', '黒羽',   '092100-kurobane'),
    ('district-092100-yuzukami', 'district', '湯津上', '092100-yuzukami')
ON CONFLICT DO NOTHING;

INSERT INTO topics (id, municipality_id, title, description, source_type, source_url, created_at) VALUES
    ('sample-092100-1', '092100', '【サンプル】放課後の子どもの居場所づくり',
     '動作確認用のサンプルです（実際の議案ではありません）。放課後や長期休みに、子どもが安心して過ごせる場所をどう増やすかを考えます。',
     'citizen', NULL, '2026-10-24T09:00:00.000Z'),
    ('sample-092100-2', '092100', '【サンプル】黒羽地域の公共交通',
     '動作確認用のサンプルです（実際の議案ではありません）。通院や買い物の移動手段として、バスや乗り合い交通のあり方を考えます。',
     'citizen', NULL, '2026-10-25T09:00:00.000Z'),
    ('sample-092100-3', '092100', '【サンプル】災害時の避難所の運営',
     '動作確認用のサンプルです（実際の議案ではありません）。大雨や地震のときに、避難所を誰がどのように運営するかを考えます。',
     'citizen', NULL, '2026-10-26T09:00:00.000Z')
ON CONFLICT DO NOTHING;

-- 締め切ったトピック（提言を試すため。ADR 0005）。同じ分野で似ているので、提言のまとまりの候補になる
INSERT INTO topics (id, municipality_id, title, description, source_type, source_url, created_at, published_at, closes_at) VALUES
    ('sample-092100-4', '092100', '【サンプル】市内の路線バスの本数',
     '動作確認用のサンプルです（実際の議案ではありません）。締め切ったトピックです。市内の路線バスの本数と、乗り合い交通のあり方を考えます。',
     'citizen', NULL, '2026-07-01T09:00:00.000Z', '2026-07-01T09:00:00.000Z', '2026-09-29T09:00:00.000Z'),
    ('sample-092100-5', '092100', '【サンプル】高校生の通学のバス',
     '動作確認用のサンプルです（実際の議案ではありません）。締め切ったトピックです。高校生の通学に使うバスの本数と運賃を考えます。',
     'citizen', NULL, '2026-07-02T09:00:00.000Z', '2026-07-02T09:00:00.000Z', '2026-09-30T09:00:00.000Z')
ON CONFLICT DO NOTHING;

INSERT INTO topic_tags (topic_id, tag_id) VALUES
    ('sample-092100-1', 'field-kosodate'),
    ('sample-092100-2', 'field-koutsu'),
    ('sample-092100-2', 'district-092100-kurobane'),
    ('sample-092100-3', 'field-bousai'),
    ('sample-092100-4', 'field-koutsu'),
    ('sample-092100-5', 'field-koutsu'),
    ('sample-092100-3', 'district-092100-yuzukami')
ON CONFLICT DO NOTHING;

-- 初期シード意見（user_id は NULL、is_seed は TRUE）。すべて承認済み
INSERT INTO statements (id, topic_id, user_id, body, status, created_at, is_seed) VALUES
    ('sample-092100-1-01', 'sample-092100-1', NULL, '学校の空き教室を放課後の居場所として使えるようにしてほしい。', 'approved', '2026-10-24T09:00:00.000Z', TRUE),
    ('sample-092100-1-02', 'sample-092100-1', NULL, '公民館で地域の大人が見守る仕組みがあると安心できる。', 'approved', '2026-10-24T09:00:00.000Z', TRUE),
    ('sample-092100-1-03', 'sample-092100-1', NULL, '利用料が高いと使えない家庭が出るので、無料か低額にすべきだ。', 'approved', '2026-10-24T09:00:00.000Z', TRUE),
    ('sample-092100-1-04', 'sample-092100-1', NULL, '新しい施設を建てるより、今ある施設の開館時間を延ばすほうがよい。', 'approved', '2026-10-24T09:00:00.000Z', TRUE),
    ('sample-092100-1-05', 'sample-092100-1', NULL, '中高生が勉強や話をできる場所も必要だ。', 'approved', '2026-10-24T09:00:00.000Z', TRUE),
    ('sample-092100-1-06', 'sample-092100-1', NULL, '居場所づくりは行政より地域のボランティアに任せたほうがうまくいく。', 'approved', '2026-10-24T09:00:00.000Z', TRUE),

    ('sample-092100-2-01', 'sample-092100-2', NULL, 'バスの本数が少なく、通院の帰りに長く待つことになる。', 'approved', '2026-10-25T09:00:00.000Z', TRUE),
    ('sample-092100-2-02', 'sample-092100-2', NULL, '予約制の乗り合いタクシーのほうが、定期バスより使いやすい。', 'approved', '2026-10-25T09:00:00.000Z', TRUE),
    ('sample-092100-2-03', 'sample-092100-2', NULL, '高校生の通学のために、朝夕の便だけでも増やしてほしい。', 'approved', '2026-10-25T09:00:00.000Z', TRUE),
    ('sample-092100-2-04', 'sample-092100-2', NULL, '利用者が少ない路線は、費用を考えて見直すべきだ。', 'approved', '2026-10-25T09:00:00.000Z', TRUE),
    ('sample-092100-2-05', 'sample-092100-2', NULL, 'スマホを使えない人のために、電話での予約も残してほしい。', 'approved', '2026-10-25T09:00:00.000Z', TRUE),

    ('sample-092100-3-01', 'sample-092100-3', NULL, '避難所の運営は、地域の住民が日ごろから訓練しておくべきだ。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),
    ('sample-092100-3-02', 'sample-092100-3', NULL, '女性や子ども向けの更衣室や授乳スペースを必ず用意してほしい。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),
    ('sample-092100-3-03', 'sample-092100-3', NULL, 'ペットと一緒に避難できる場所があると、避難をためらう人が減る。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),
    ('sample-092100-3-04', 'sample-092100-3', NULL, '避難所の空き状況をスマホで見られるようにしてほしい。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),
    ('sample-092100-3-05', 'sample-092100-3', NULL, '外国籍の住民にも伝わるよう、案内をやさしい日本語でも出すべきだ。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),
    ('sample-092100-3-06', 'sample-092100-3', NULL, '備蓄品は市がすべて用意するより、各家庭の備えを増やすほうが現実的だ。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),
    ('sample-092100-3-07', 'sample-092100-3', NULL, '車中泊をする人の健康状態も確認できる仕組みがほしい。', 'approved', '2026-10-26T09:00:00.000Z', TRUE),

    ('sample-092100-4-01', 'sample-092100-4', NULL, 'バスの時刻表を、駅や病院でも分かりやすく掲示してほしい。', 'approved', '2026-07-01T09:00:00.000Z', TRUE),
    ('sample-092100-4-02', 'sample-092100-4', NULL, '利用の少ない路線は廃止して、乗り合いタクシーに切り替えるべきだ。', 'approved', '2026-07-01T09:00:00.000Z', TRUE),
    ('sample-092100-4-03', 'sample-092100-4', NULL, '赤字でも、路線バスは市が守るべき公共サービスだ。', 'approved', '2026-07-01T09:00:00.000Z', TRUE),
    ('sample-092100-4-04', 'sample-092100-4', NULL, '運転手の確保のために、市が待遇の改善を支援すべきだ。', 'approved', '2026-07-01T09:00:00.000Z', TRUE),
    ('sample-092100-4-05', 'sample-092100-4', NULL, 'バスの本数を増やすより、停留所を増やすほうが助かる。', 'approved', '2026-07-01T09:00:00.000Z', TRUE),

    ('sample-092100-5-01', 'sample-092100-5', NULL, '部活動の帰りに使える、遅い時間の便がほしい。', 'approved', '2026-07-02T09:00:00.000Z', TRUE),
    ('sample-092100-5-02', 'sample-092100-5', NULL, '通学定期の運賃を、市が補助すべきだ。', 'approved', '2026-07-02T09:00:00.000Z', TRUE),
    ('sample-092100-5-03', 'sample-092100-5', NULL, '通学は家族の送迎や自転車で足りるので、バスに予算をかけなくてよい。', 'approved', '2026-07-02T09:00:00.000Z', TRUE),
    ('sample-092100-5-04', 'sample-092100-5', NULL, '雨や雪の日だけでも、臨時の便を出してほしい。', 'approved', '2026-07-02T09:00:00.000Z', TRUE),
    ('sample-092100-5-05', 'sample-092100-5', NULL, '高校と相談して、始業の時刻に合わせたダイヤにしてほしい。', 'approved', '2026-07-02T09:00:00.000Z', TRUE)
ON CONFLICT DO NOTHING;
