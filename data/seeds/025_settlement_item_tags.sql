-- SPDX-License-Identifier: AGPL-3.0-only
-- 決算カードの目的別歳出の費目と、分野タグ（020_field_tags.sql）の対応（全自治体共通）。
-- トピックの詳細に「この分野の市のお金」として、トピックの分野タグに対応する費目の数字を出す。
-- 費目は分野よりずっと広い（例: 民生費には福祉と子育ての両方が入る）ので、対応はおおまか。画面でもそう注記する
-- 対応の無い分野（防犯・交通安全、交通・移動、公共施設、人権・共生、財政・税など）は、いくつもの費目にまたがるので付けない
INSERT INTO settlement_item_tags (category, item_key, tag_id) VALUES
    ('expenditure_purpose', '議会費',       'field-gikai'),
    ('expenditure_purpose', '総務費',       'field-gyousei'),
    ('expenditure_purpose', '民生費',       'field-fukushi'),
    ('expenditure_purpose', '民生費',       'field-kosodate'),
    ('expenditure_purpose', '衛生費',       'field-iryou'),
    ('expenditure_purpose', '衛生費',       'field-kankyou'),
    ('expenditure_purpose', '労働費',       'field-sangyou'),
    ('expenditure_purpose', '農林水産業費', 'field-nourin'),
    ('expenditure_purpose', '商工費',       'field-sangyou'),
    ('expenditure_purpose', '商工費',       'field-kankou'),
    ('expenditure_purpose', '土木費',       'field-infra'),
    ('expenditure_purpose', '土木費',       'field-machidukuri'),
    ('expenditure_purpose', '土木費',       'field-sumai'),
    ('expenditure_purpose', '消防費',       'field-bousai'),
    ('expenditure_purpose', '教育費',       'field-kosodate'),
    ('expenditure_purpose', '教育費',       'field-bunka'),
    ('expenditure_purpose', '災害復旧費',   'field-bousai')
ON CONFLICT DO NOTHING;
-- 公債費（借金の返済）は「財政・税」の話題（税の仕組みなど）と関係が薄く、並べると誤解を招くので対応させない。
-- 財政全体は「まちのお金」のページで見せる（2026-10-27）
