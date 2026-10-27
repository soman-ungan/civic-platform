// SPDX-License-Identifier: AGPL-3.0-only

import { FinanceDonut } from "@/components/finance-donut";
import { InfoTip } from "@/components/info-tip";
import { NATURE_GROUPS, natureGroupOf, termNote } from "@/lib/finance-terms";
import { roughYen, shareText } from "@/lib/money";
import type { FinanceAmount } from "@/lib/types";

/**
 * お金の使い方で見る（性質別の歳出を5区分にまとめたもの）。
 * 割合を円グラフで見せ、その下に区分ごとの行を並べる。区分の行を開くと中身の費目が出る。
 * 5色は系列の色の1〜5番目。円の中に文字は入れず、行の色見本と文字で示す（色だけに頼らない）
 */
export function FinanceNature({ items }: { items: FinanceAmount[] }) {
  const total = items.reduce((a, i) => a + (i.value ?? 0), 0);
  const groups = NATURE_GROUPS.map((g, i) => {
    const members = items.filter(
      (item) => natureGroupOf(item.key) === g.id && (item.value ?? 0) > 0,
    );
    const sum = (f: (i: FinanceAmount) => number | null) =>
      members.reduce((a, m) => a + (f(m) ?? 0), 0);
    return {
      ...g,
      color: `var(--series-${i + 1})`,
      members: members.sort((a, b) => (b.value ?? 0) - (a.value ?? 0)),
      value: sum((m) => m.value),
      perCapita: sum((m) => m.per_capita),
    };
  });
  if (total <= 0) return null;

  return (
    <div>
      <FinanceDonut
        slices={groups
          .filter((g) => g.value > 0)
          .map((g) => ({
            key: g.id,
            label: g.label,
            value: g.value,
            valueText: roughYen(g.perCapita),
            shareText: shareText(g.value / total),
            color: g.color,
          }))}
      />
      <ul className="flex flex-col">
        {groups.map((g) => (
          <li
            key={g.id}
            className="border-t border-zinc-100 first:border-t-0 dark:border-zinc-800"
          >
            <details className="group/row">
              <summary className="flex min-h-11 cursor-pointer list-none items-center justify-between gap-2 text-sm [&::-webkit-details-marker]:hidden">
                <span className="flex items-center gap-1.5 font-medium">
                  <span
                    aria-hidden
                    className="inline-block h-2.5 w-2.5 rounded-sm"
                    style={{ background: g.color }}
                  />
                  {g.label}
                  <InfoTip label={g.label} note={g.note} />
                </span>
                <span className="flex items-center gap-2 tabular-nums text-zinc-700 dark:text-zinc-300">
                  {roughYen(g.perCapita)}
                  <span className="text-xs text-zinc-500">
                    {shareText(g.value / total)}
                  </span>
                  <span
                    aria-hidden
                    className="text-xs text-zinc-400 transition-transform group-open/row:rotate-180"
                  >
                    ▼
                  </span>
                </span>
              </summary>
              <ul className="mb-3 ml-4 flex flex-col gap-1.5 border-l border-zinc-200 pl-3 dark:border-zinc-700">
                {g.members.length === 0 && (
                  <li className="text-xs text-zinc-500">
                    この年度はありません
                  </li>
                )}
                {g.members.map((m) => {
                  const note = termNote("nature", m.key);
                  return (
                    <li
                      key={m.key}
                      className="flex items-center justify-between gap-2 text-xs"
                    >
                      <span className="flex items-center gap-1">
                        {m.key}
                        {note && <InfoTip label={m.key} note={note} />}
                      </span>
                      <span className="tabular-nums text-zinc-600 dark:text-zinc-400">
                        {m.per_capita !== null ? roughYen(m.per_capita) : "—"}
                      </span>
                    </li>
                  );
                })}
              </ul>
            </details>
          </li>
        ))}
      </ul>
    </div>
  );
}
