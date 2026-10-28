// SPDX-License-Identifier: AGPL-3.0-only

// 予算の主な事業の表示に使うもの

import type { BudgetProject } from "@/lib/api/types/finances";

/** 色を付ける分野の数。残りは「ほかの分野」にまとめる（dataviz: 色は8つまで、順番は固定） */
export const COLORED_FIELDS = 7;
/** 「ほかの分野」を開いたときの、ツリーマップと一覧の絞り込みの値 */
export const OTHER_FIELDS = "__other";
/** 「ほかの分野」の色（系列ではないので灰色） */
export const OTHER_FIELD_COLOR = "var(--treemap-other)";

/** 新規・拡充の印の文字 */
export const FLAG_LABEL: Record<NonNullable<BudgetProject["flag"]>, string> = {
  new: "新規",
  expanded: "拡充",
};

export type FieldGroup = {
  id: string;
  name: string;
  /** 最初の分野として数えた事業の合計（千円）。ツリーマップの面積に使う */
  amount: number;
  /** その分野が付いた事業の数（2つ目の分野を含む） */
  count: number;
  color: string;
};

/**
 * 分野ごとのまとまり（金額の大きい順）。面積は同じお金を2回描かないように、事業の最初の分野だけで数える。
 * 色は大きい順に `--series-1..7` で、残りの分野は「ほかの分野」の灰色
 */
export function fieldGroups(projects: BudgetProject[]): FieldGroup[] {
  const byId = new Map<string, FieldGroup>();
  for (const p of projects) {
    p.tags.forEach((t, i) => {
      const g = byId.get(t.id) ?? {
        id: t.id,
        name: t.name,
        amount: 0,
        count: 0,
        color: OTHER_FIELD_COLOR,
      };
      if (i === 0) g.amount += p.amount;
      g.count += 1;
      byId.set(t.id, g);
    });
  }
  const groups = [...byId.values()].sort(
    (a, b) => b.amount - a.amount || b.count - a.count,
  );
  groups.forEach((g, i) => {
    if (i < COLORED_FIELDS) g.color = `var(--series-${i + 1})`;
  });
  return groups;
}

/** 「ほかの分野」にまとめた分野の ID */
export function otherFieldIds(groups: FieldGroup[]): Set<string> {
  return new Set(groups.slice(COLORED_FIELDS).map((g) => g.id));
}
