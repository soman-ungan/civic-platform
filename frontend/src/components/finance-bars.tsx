// SPDX-License-Identifier: AGPL-3.0-only

import { FinanceDonut } from "@/components/finance-donut";
import { InfoTip } from "@/components/info-tip";
import { type TermKind, termNote } from "@/lib/finance-terms";
import { roughYen, shareText, thousandYen, yen } from "@/lib/money";
import type { FinanceAmount } from "@/lib/types";

/** 円グラフの区切りの数の上限（dataviz: 6つまで） */
const DONUT_SLICES = 6;

/**
 * 費目ごとの横棒（大きい順）。量の比較なので1色（dataviz: sequential は1色）。
 * 棒ごとに1人あたりと割合を文字でも出すので、色や長さだけに頼らない（表の代わりにもなる）。
 * 費目の名前の横の ⓘ で、短い説明を出す。`limit` を超える費目は「その他」にまとめる。
 * `donut` のときは上に円グラフを出す。円の区切りは6つまで（dataviz）なので、大きい順の5つと「その他」にし、
 * 一覧には全部の費目を残して、円と同じ色の見本を付ける（6番目からは「その他」の色）
 */
export function FinanceBars({
  items,
  kind,
  limit,
  donut = false,
}: {
  items: FinanceAmount[];
  /** 説明の辞書（lib/finance-terms.ts） */
  kind: TermKind;
  limit?: number;
  donut?: boolean;
}) {
  const sorted = items
    .filter((i): i is FinanceAmount & { value: number } => (i.value ?? 0) > 0)
    .sort((a, b) => b.value - a.value);
  const shown = limit ? sorted.slice(0, limit) : sorted;
  const rest = limit ? sorted.slice(limit) : [];
  const rows = shown.map((r) => ({ ...r, note: termNote(kind, r.key) }));
  if (rest.length > 0) {
    const sum = (f: (i: FinanceAmount) => number | null) =>
      rest.reduce((acc, i) => acc + (f(i) ?? 0), 0);
    rows.push({
      key: `その他（${rest.length}項目）`,
      value: sum((i) => i.value),
      per_capita: sum((i) => i.per_capita),
      share: sum((i) => i.share),
      note: null,
    });
  }
  const max = Math.max(...rows.map((r) => r.value), 1);
  const colorOf = (i: number) =>
    `var(--series-${Math.min(i, DONUT_SLICES - 1) + 1})`;
  const total = rows.reduce((a, r) => a + r.value, 0);
  const perCapitaTotal = rows.reduce((a, r) => a + (r.per_capita ?? 0), 0);
  const slices = rows.slice(0, DONUT_SLICES - 1).map((r, i) => ({
    key: r.key,
    label: r.key,
    value: r.value,
    valueText: r.per_capita !== null ? roughYen(r.per_capita) : "—",
    shareText: shareText(r.value / total),
    color: colorOf(i),
  }));
  const tail = rows.slice(DONUT_SLICES - 1);
  if (tail.length > 0) {
    const value = tail.reduce((a, r) => a + r.value, 0);
    slices.push({
      key: "__other",
      label: `その他（${tail.length}項目）`,
      value,
      valueText: roughYen(tail.reduce((a, r) => a + (r.per_capita ?? 0), 0)),
      shareText: shareText(value / total),
      color: colorOf(DONUT_SLICES - 1),
    });
  }

  return (
    <>
      {donut && (
        <FinanceDonut
          slices={slices}
          totalLabel="1人あたり"
          totalText={roughYen(perCapitaTotal)}
        />
      )}
      <ul className="flex flex-col gap-2.5">
        {rows.map((row, i) => (
          <li key={row.key}>
            <div className="flex items-baseline justify-between gap-2 text-sm">
              <span className="flex items-center gap-1 font-medium">
                {donut && (
                  <span
                    aria-hidden
                    className="mr-0.5 inline-block h-2.5 w-2.5 shrink-0 rounded-sm"
                    style={{ background: colorOf(i) }}
                  />
                )}
                {row.key}
                {row.note && <InfoTip label={row.key} note={row.note} />}
              </span>
              <span className="tabular-nums text-zinc-700 dark:text-zinc-300">
                {row.per_capita !== null ? roughYen(row.per_capita) : "—"}
                <span className="ml-1.5 text-xs text-zinc-500">
                  {row.share !== null && shareText(row.share)}
                </span>
              </span>
            </div>
            <div
              className="mt-1 h-2 rounded-full bg-zinc-100 dark:bg-zinc-800"
              title={`${row.key}: ${thousandYen(row.value)}${
                row.per_capita !== null
                  ? `（1人あたり ${yen(row.per_capita)}）`
                  : ""
              }`}
            >
              <div
                className="h-2 rounded-full bg-sky-600 dark:bg-sky-400"
                style={{ width: `${(row.value / max) * 100}%` }}
              />
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}
