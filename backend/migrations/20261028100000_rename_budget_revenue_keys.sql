-- SPDX-License-Identifier: AGPL-3.0-only
-- 予算の歳入の費目の名前を、決算カードの名前にそろえる（backend/src/budget/mod.rs の revenue_key と同じ対応）。
-- 取り込みのときにそろえるようにしたので、それより前に取り込んだ予算を直す
UPDATE budget_items SET item_key = '地方税'
    WHERE category = 'revenue' AND item_key IN ('市税', '町税', '村税', '区税');
UPDATE budget_items SET item_key = '地方債'
    WHERE category = 'revenue' AND item_key IN ('市債', '町債', '村債', '区債');
UPDATE budget_items SET item_key = '都道府県支出金'
    WHERE category = 'revenue' AND item_key IN ('県支出金', '都支出金', '道支出金', '府支出金');
UPDATE budget_items SET item_key = '分担金・負担金'
    WHERE category = 'revenue' AND item_key = '分担金及び負担金';
UPDATE budget_items SET item_key = '地方特例交付金等'
    WHERE category = 'revenue' AND item_key = '地方特例交付金';
UPDATE budget_items SET item_key = '自動車税環境性能割交付金'
    WHERE category = 'revenue' AND item_key = '環境性能割交付金';
UPDATE budget_items SET item_key = '使用料・手数料'
    WHERE category = 'revenue' AND item_key = '使用料及び手数料';
