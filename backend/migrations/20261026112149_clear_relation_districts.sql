-- SPDX-License-Identifier: AGPL-3.0-only
-- 参加者の地区は集めない（市民を地区で分けると分断を煽るため。docs/adr/0002-participant-attributes.md）。
-- すでに入っている値を消す。列は、マージ済みのマイグレーションを書き換えないルールに従って残す（将来削除する候補）。
UPDATE user_municipality_relations SET district_id = NULL;
