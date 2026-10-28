// SPDX-License-Identifier: AGPL-3.0-only

import { type Unit, amountText } from "@/lib/money";
import type { FinanceAmount } from "@/lib/types";

/**
 * 同じ年度の予算（当初予算）と決算を、目的別の費目ごとに並べる。
 * 2本の横棒（予算は薄く、決算は濃く）を同じ物差しで描き、差を文字でも出す
 */
export function BudgetComparison({
  budget,
  settlement,
  unit,
}: {
  budget: FinanceAmount[];
  settlement: FinanceAmount[];
  /** 1人あたりか総額か（ヘッダーの切り替え。1人あたりは、それぞれの人口で割った額） */
  unit: Unit;
}) {
  const keys = [
    ...new Set([...budget, ...settlement].map((i) => i.key)),
  ].filter((k) => k !== "予備費");
  const find = (items: FinanceAmount[], key: string) =>
    items.find((i) => i.key === key);
  const value = (items: FinanceAmount[], key: string) =>
    find(items, key)?.value ?? 0;
  const text = (items: FinanceAmount[], key: string) => {
    const item = find(items, key);
    return amountText(unit, item?.value ?? 0, item?.per_capita ?? 0);
  };
  const rows = keys
    .map((key) => ({
      key,
      budget: value(budget, key),
      settlement: value(settlement, key),
    }))
    .filter((r) => r.budget > 0 || r.settlement > 0)
    .sort((a, b) => b.budget - a.budget);
  const max = Math.max(...rows.flatMap((r) => [r.budget, r.settlement]), 1);

  return (
    <div>
      <ul className="mb-3 flex gap-4 text-xs text-zinc-600 dark:text-zinc-400">
        <li className="flex items-center gap-1.5">
          <span
            aria-hidden
            className="inline-block h-2 w-3 rounded-sm bg-sky-300 dark:bg-sky-800"
          />
          予算（年度の初めの見込み）
        </li>
        <li className="flex items-center gap-1.5">
          <span
            aria-hidden
            className="inline-block h-2 w-3 rounded-sm bg-sky-600 dark:bg-sky-400"
          />
          決算（実際に使った額）
        </li>
      </ul>
      <ul className="flex flex-col gap-3">
        {rows.map((r) => {
          const diff =
            r.budget > 0 ? (r.settlement - r.budget) / r.budget : null;
          return (
            <li key={r.key}>
              <div className="flex items-baseline justify-between gap-2 text-sm">
                <span className="font-medium">{r.key}</span>
                <span className="text-xs tabular-nums text-zinc-600 dark:text-zinc-400">
                  {text(budget, r.key)} → {text(settlement, r.key)}
                  {diff !== null && (
                    <span className="ml-1.5">
                      （{diff >= 0 ? "+" : ""}
                      {(diff * 100).toLocaleString("ja-JP", {
                        maximumFractionDigits: 1,
                      })}
                      %）
                    </span>
                  )}
                </span>
              </div>
              <div className="mt-1 flex flex-col gap-0.5">
                <div
                  className="h-1.5 rounded-full bg-sky-300 dark:bg-sky-800"
                  style={{ width: `${(r.budget / max) * 100}%` }}
                />
                <div
                  className="h-1.5 rounded-full bg-sky-600 dark:bg-sky-400"
                  style={{ width: `${(r.settlement / max) * 100}%` }}
                />
              </div>
            </li>
          );
        })}
      </ul>
      <p className="mt-3 text-xs leading-5 text-zinc-500">
        決算には、年度の途中の補正予算の分も入るので、当初予算より大きくなることがよくあります。
      </p>
    </div>
  );
}
