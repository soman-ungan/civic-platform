-- SPDX-License-Identifier: AGPL-3.0-only
-- 公人の名前の読み。議員の一覧を五十音で並べるためだけに使い、画面には出さない（ADR 0008）
ALTER TABLE public_officials ADD COLUMN name_kana TEXT;
