-- SPDX-License-Identifier: AGPL-3.0-or-later
-- 開発・動作確認用のサンプルデータ（本番には投入しない。`backend seed --dev` でのみ適用）
-- 内容はすべて架空。実際の議案・発言ではない。タイトルには必ず【サンプル】を付ける。
-- 前提: data/seeds/ の大田原市（092100）と地区が登録済みであること。

-- 分野タグ（全自治体共通の語彙）
INSERT INTO tags (id, kind, name) VALUES
    ('field-kosodate', 'field', '子育て'),
    ('field-koutsu',   'field', '交通'),
    ('field-bousai',   'field', '防災')
ON CONFLICT DO NOTHING;

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

INSERT INTO topic_tags (topic_id, tag_id) VALUES
    ('sample-092100-1', 'field-kosodate'),
    ('sample-092100-2', 'field-koutsu'),
    ('sample-092100-2', 'district-092100-kurobane'),
    ('sample-092100-3', 'field-bousai'),
    ('sample-092100-3', 'district-092100-yuzukami')
ON CONFLICT DO NOTHING;

-- 初期シード意見（user_id は NULL）。すべて承認済み
INSERT INTO statements (id, topic_id, user_id, body, status, created_at) VALUES
    ('sample-092100-1-01', 'sample-092100-1', NULL, '学校の空き教室を放課後の居場所として使えるようにしてほしい。', 'approved', '2026-10-24T09:00:00.000Z'),
    ('sample-092100-1-02', 'sample-092100-1', NULL, '公民館で地域の大人が見守る仕組みがあると安心できる。', 'approved', '2026-10-24T09:00:00.000Z'),
    ('sample-092100-1-03', 'sample-092100-1', NULL, '利用料が高いと使えない家庭が出るので、無料か低額にすべきだ。', 'approved', '2026-10-24T09:00:00.000Z'),
    ('sample-092100-1-04', 'sample-092100-1', NULL, '新しい施設を建てるより、今ある施設の開館時間を延ばすほうがよい。', 'approved', '2026-10-24T09:00:00.000Z'),
    ('sample-092100-1-05', 'sample-092100-1', NULL, '中高生が勉強や話をできる場所も必要だ。', 'approved', '2026-10-24T09:00:00.000Z'),
    ('sample-092100-1-06', 'sample-092100-1', NULL, '居場所づくりは行政より地域のボランティアに任せたほうがうまくいく。', 'approved', '2026-10-24T09:00:00.000Z'),

    ('sample-092100-2-01', 'sample-092100-2', NULL, 'バスの本数が少なく、通院の帰りに長く待つことになる。', 'approved', '2026-10-25T09:00:00.000Z'),
    ('sample-092100-2-02', 'sample-092100-2', NULL, '予約制の乗り合いタクシーのほうが、定期バスより使いやすい。', 'approved', '2026-10-25T09:00:00.000Z'),
    ('sample-092100-2-03', 'sample-092100-2', NULL, '高校生の通学のために、朝夕の便だけでも増やしてほしい。', 'approved', '2026-10-25T09:00:00.000Z'),
    ('sample-092100-2-04', 'sample-092100-2', NULL, '利用者が少ない路線は、費用を考えて見直すべきだ。', 'approved', '2026-10-25T09:00:00.000Z'),
    ('sample-092100-2-05', 'sample-092100-2', NULL, 'スマホを使えない人のために、電話での予約も残してほしい。', 'approved', '2026-10-25T09:00:00.000Z'),

    ('sample-092100-3-01', 'sample-092100-3', NULL, '避難所の運営は、地域の住民が日ごろから訓練しておくべきだ。', 'approved', '2026-10-26T09:00:00.000Z'),
    ('sample-092100-3-02', 'sample-092100-3', NULL, '女性や子ども向けの更衣室や授乳スペースを必ず用意してほしい。', 'approved', '2026-10-26T09:00:00.000Z'),
    ('sample-092100-3-03', 'sample-092100-3', NULL, 'ペットと一緒に避難できる場所があると、避難をためらう人が減る。', 'approved', '2026-10-26T09:00:00.000Z'),
    ('sample-092100-3-04', 'sample-092100-3', NULL, '避難所の空き状況をスマホで見られるようにしてほしい。', 'approved', '2026-10-26T09:00:00.000Z'),
    ('sample-092100-3-05', 'sample-092100-3', NULL, '外国籍の住民にも伝わるよう、案内をやさしい日本語でも出すべきだ。', 'approved', '2026-10-26T09:00:00.000Z'),
    ('sample-092100-3-06', 'sample-092100-3', NULL, '備蓄品は市がすべて用意するより、各家庭の備えを増やすほうが現実的だ。', 'approved', '2026-10-26T09:00:00.000Z'),
    ('sample-092100-3-07', 'sample-092100-3', NULL, '車中泊をする人の健康状態も確認できる仕組みがほしい。', 'approved', '2026-10-26T09:00:00.000Z')
ON CONFLICT DO NOTHING;
