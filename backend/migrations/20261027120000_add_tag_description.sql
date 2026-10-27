-- SPDX-License-Identifier: AGPL-3.0-only
-- 分野タグの定義（何を含み、何を含まないか）。自動タグ付けの指示文に入れる
-- 判断基準は docs/design/tagging-criteria.md
ALTER TABLE tags ADD COLUMN description TEXT;
