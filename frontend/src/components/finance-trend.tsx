// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { fiscalYearLabel, roughYen } from "@/lib/money";
import type { Finances } from "@/lib/types";

const WIDTH = 320;
const HEIGHT = 150;
const PAD_X = 16;
const PAD_TOP = 12;
const PAD_BOTTOM = 22;
const TOTAL = "歳出の合計";

/** 2015 → H27、2020 → R2（軸の目盛り用の短い表記） */
function shortYear(year: number): string {
  return year >= 2019 ? `R${year - 2018}` : `H${year - 1988}`;
}

/**
 * 1人あたりの歳出の推移（1系列の折れ線）。費目を選ぶと、その費目の推移に切り替わる。
 * 縦軸は 0 から（量の比較なので）。触れた年度の値を上に出す（スマホでも読めるように、タップでも選べる）
 */
export function FinanceTrend({
  trend,
  keys,
}: {
  trend: Finances["trend"];
  /** 選べる費目（大きい順） */
  keys: string[];
}) {
  const [selected, setSelected] = useState(TOTAL);
  const [active, setActive] = useState<number | null>(null);

  const points = trend.flatMap((t) => {
    const value =
      selected === TOTAL
        ? Object.values(t.expenditure_purpose).reduce<number>(
            (a, v) => a + (v ?? 0),
            0,
          )
        : t.expenditure_purpose[selected];
    if (value === null || value === undefined || !t.population) return [];
    return [{ year: t.fiscal_year, perCapita: (value * 1000) / t.population }];
  });
  if (points.length < 2) return null;

  const max = Math.max(...points.map((p) => p.perCapita)) * 1.1;
  const x = (i: number) =>
    PAD_X + (i * (WIDTH - PAD_X * 2)) / (points.length - 1);
  const y = (v: number) =>
    PAD_TOP + (1 - v / max) * (HEIGHT - PAD_TOP - PAD_BOTTOM);
  const shownIndex = active ?? points.length - 1;
  const shown = points[shownIndex];
  const path = points
    .map((p, i) => `${i === 0 ? "M" : "L"}${x(i)},${y(p.perCapita)}`)
    .join(" ");

  const pick = (clientX: number, rect: DOMRect) => {
    const px = ((clientX - rect.left) / rect.width) * WIDTH;
    const i = Math.round(
      ((px - PAD_X) / (WIDTH - PAD_X * 2)) * (points.length - 1),
    );
    setActive(Math.min(points.length - 1, Math.max(0, i)));
  };

  return (
    <div>
      <div className="mb-3 flex flex-wrap gap-1.5">
        {[TOTAL, ...keys].map((k) => (
          <button
            key={k}
            type="button"
            onClick={() => {
              setSelected(k);
              setActive(null);
            }}
            aria-pressed={selected === k}
            className={`min-h-9 rounded-full border px-3 text-xs ${
              selected === k
                ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
                : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
            }`}
          >
            {k}
          </button>
        ))}
      </div>
      <p className="text-sm" aria-live="polite">
        <span className="text-zinc-500">{fiscalYearLabel(shown.year)}</span>{" "}
        <span className="font-semibold tabular-nums">
          {roughYen(shown.perCapita)}
        </span>
        <span className="text-xs text-zinc-500">（1人あたり）</span>
      </p>
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        data-swipe-ignore
        className="mt-1 w-full touch-none select-none"
        role="img"
        aria-label={`${selected}の1人あたりの推移`}
        onPointerMove={(e) =>
          pick(e.clientX, e.currentTarget.getBoundingClientRect())
        }
        onPointerDown={(e) =>
          pick(e.clientX, e.currentTarget.getBoundingClientRect())
        }
        onPointerLeave={() => setActive(null)}
      >
        <line
          x1={PAD_X}
          x2={WIDTH - PAD_X}
          y1={HEIGHT - PAD_BOTTOM}
          y2={HEIGHT - PAD_BOTTOM}
          className="stroke-zinc-300 dark:stroke-zinc-700"
          strokeWidth={1}
        />
        <line
          x1={x(shownIndex)}
          x2={x(shownIndex)}
          y1={PAD_TOP}
          y2={HEIGHT - PAD_BOTTOM}
          className="stroke-zinc-300 dark:stroke-zinc-600"
          strokeWidth={1}
          strokeDasharray="3 3"
        />
        <path
          d={path}
          fill="none"
          strokeWidth={2}
          strokeLinejoin="round"
          className="stroke-sky-600 dark:stroke-sky-400"
        />
        {points.map((p, i) => (
          <circle
            key={p.year}
            cx={x(i)}
            cy={y(p.perCapita)}
            r={i === shownIndex ? 4.5 : 3}
            strokeWidth={2}
            className="fill-sky-600 stroke-white dark:fill-sky-400 dark:stroke-zinc-950"
          />
        ))}
        {points.map((p, i) =>
          i === 0 || i === points.length - 1 || i % 3 === 0 ? (
            <text
              key={p.year}
              x={x(i)}
              y={HEIGHT - 6}
              textAnchor="middle"
              className="fill-zinc-500 text-[10px]"
            >
              {shortYear(p.year)}
            </text>
          ) : null,
        )}
      </svg>
      <details className="mt-2 text-xs text-zinc-600 dark:text-zinc-400">
        <summary className="cursor-pointer py-1">数値の一覧</summary>
        <table className="mt-1 w-full tabular-nums">
          <tbody>
            {points.map((p) => (
              <tr
                key={p.year}
                className="border-t border-zinc-100 dark:border-zinc-800"
              >
                <td className="py-1">{fiscalYearLabel(p.year)}</td>
                <td className="py-1 text-right">{roughYen(p.perCapita)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </details>
    </div>
  );
}
