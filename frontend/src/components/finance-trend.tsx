// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { termNote } from "@/lib/finance-terms";
import { fiscalYearLabel, roughYen, thousandYen } from "@/lib/money";
import type { Finances } from "@/lib/types";

const WIDTH = 320;
const HEIGHT = 190;
const PAD_LEFT = 38;
const PAD_RIGHT = 4;
const PAD_TOP = 8;
const PAD_BOTTOM = 20;
/** 系列の色は8色まで（dataviz）。最新の年度の上位7費目と「その他」 */
const TOP_SERIES = 7;
const OTHER = "その他";
/** 積み上げの段の間のすき間（px） */
const GAP = 2;

type Mode = "per_capita" | "total";

/** 2015 → H27、2020 → R2（軸の目盛り用の短い表記） */
function shortYear(year: number): string {
  return year >= 2019 ? `R${year - 2018}` : `H${year - 1988}`;
}

/** 目盛りの間隔（1・2・2.5・5 × 10^n）を、4目盛りほどになるように選ぶ */
function niceStep(max: number): number {
  const raw = max / 4;
  const base = 10 ** Math.floor(Math.log10(raw));
  return [1, 2, 2.5, 5, 10].map((m) => m * base).find((s) => s >= raw) ?? raw;
}

/**
 * 歳出の推移（目的別の積み上げの縦棒）。横軸は年度、縦軸は金額（0 から）。
 * 1人あたり（円）と総額を切り替えられ、費目のチップで表示・非表示を切り替える（最初は全部 = 歳出の合計）。
 * 色は費目に付け、表示を切り替えても変えない。最初はヘッダーで選んだ年度をハイライトし、
 * 棒に触れると、その年度に移って内訳を下に出す
 */
export function FinanceTrend({
  trend,
  latestOrder,
  selectedYear,
}: {
  trend: Finances["trend"];
  /** 最新の年度の目的別歳出の費目（大きい順） */
  latestOrder: string[];
  /** ヘッダーで選んでいる年度 */
  selectedYear: number;
}) {
  const [mode, setMode] = useState<Mode>("per_capita");
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  /** 棒に触れて選んだ年度。触れるまではヘッダーの年度 */
  const [touchedYear, setTouchedYear] = useState<number | null>(null);

  const top = latestOrder.slice(0, TOP_SERIES);
  const series = [...top, OTHER].map((key, i) => ({
    key,
    color: `var(--series-${i + 1})`,
  }));
  const years = trend.filter((t) => t.population);
  if (years.length < 2) return null;

  /** 年度・系列ごとの値（mode の単位） */
  const valueOf = (t: Finances["trend"][number], key: string): number => {
    const raw =
      key === OTHER
        ? Object.entries(t.expenditure_purpose)
            .filter(([k]) => !top.includes(k))
            .reduce((a, [, v]) => a + (v ?? 0), 0)
        : (t.expenditure_purpose[key] ?? 0);
    return mode === "per_capita" ? (raw * 1000) / (t.population ?? 1) : raw;
  };
  const format = (v: number) =>
    mode === "per_capita" ? roughYen(v) : thousandYen(v);
  const visible = series.filter((s) => !hidden.has(s.key));
  const stacks = years.map((t) => ({
    year: t.fiscal_year,
    parts: visible.map((s) => ({ ...s, value: valueOf(t, s.key) })),
  }));
  const totals = stacks.map((s) => s.parts.reduce((a, p) => a + p.value, 0));
  const step = niceStep(Math.max(...totals, 1));
  const max = step * Math.ceil(Math.max(...totals, 1) / step);
  const ticks = Array.from(
    { length: Math.round(max / step) + 1 },
    (_, i) => i * step,
  );

  const innerW = WIDTH - PAD_LEFT - PAD_RIGHT;
  const innerH = HEIGHT - PAD_TOP - PAD_BOTTOM;
  const band = innerW / stacks.length;
  const barW = band * 0.64;
  const y = (v: number) => PAD_TOP + innerH * (1 - v / max);
  const wanted = touchedYear ?? selectedYear;
  const found = stacks.findIndex((s) => s.year === wanted);
  const shownIndex = found >= 0 ? found : stacks.length - 1;
  const shown = stacks[shownIndex];

  const tickLabel = (v: number) =>
    v === 0
      ? "0"
      : mode === "per_capita"
        ? `${(v / 10_000).toLocaleString("ja-JP")}万`
        : `${(v / 100_000).toLocaleString("ja-JP")}億`;

  function toggle(key: string) {
    setHidden((h) => {
      const next = new Set(h);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }

  return (
    <div>
      <div className="mb-2 flex items-center justify-between gap-2">
        <div
          role="group"
          aria-label="単位"
          className="inline-flex rounded-full border border-zinc-300 p-0.5 text-xs dark:border-zinc-700"
        >
          {(
            [
              ["per_capita", "1人あたり"],
              ["total", "総額"],
            ] as const
          ).map(([m, label]) => (
            <button
              key={m}
              type="button"
              aria-pressed={mode === m}
              onClick={() => setMode(m)}
              className={`min-h-8 rounded-full px-3 ${
                mode === m
                  ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900"
                  : "text-zinc-600 dark:text-zinc-400"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="flex gap-1.5">
          <button
            type="button"
            onClick={() => setHidden(new Set())}
            disabled={hidden.size === 0}
            className="inline-flex min-h-8 items-center rounded-full border border-zinc-300 px-2.5 text-xs whitespace-nowrap text-zinc-700 disabled:opacity-40 dark:border-zinc-700 dark:text-zinc-300"
          >
            すべて選ぶ
          </button>
          <button
            type="button"
            onClick={() => setHidden(new Set(series.map((s) => s.key)))}
            disabled={visible.length === 0}
            className="inline-flex min-h-8 items-center rounded-full border border-zinc-300 px-2.5 text-xs whitespace-nowrap text-zinc-700 disabled:opacity-40 dark:border-zinc-700 dark:text-zinc-300"
          >
            すべて外す
          </button>
        </div>
      </div>

      <nav
        aria-label="表示する費目"
        data-swipe-ignore
        className="-mx-4 mb-3 overflow-x-auto px-4 [scrollbar-width:none]"
      >
        <ul className="flex w-max gap-1.5">
          {series.map((s) => {
            const on = !hidden.has(s.key);
            return (
              <li key={s.key}>
                <button
                  type="button"
                  aria-pressed={on}
                  onClick={() => toggle(s.key)}
                  title={termNote("purpose", s.key) ?? undefined}
                  className={`inline-flex min-h-9 items-center gap-1.5 rounded-full border px-2.5 text-xs whitespace-nowrap ${
                    on
                      ? "border-zinc-400 text-zinc-800 dark:border-zinc-500 dark:text-zinc-200"
                      : "border-zinc-200 text-zinc-400 line-through dark:border-zinc-800 dark:text-zinc-600"
                  }`}
                >
                  <span
                    aria-hidden
                    className="inline-block h-2.5 w-2.5 rounded-sm"
                    style={{
                      background: on ? s.color : "transparent",
                      outline: `1.5px solid ${s.color}`,
                    }}
                  />
                  {s.key === OTHER
                    ? `その他（${Math.max(latestOrder.length - TOP_SERIES, 0)}項目）`
                    : s.key}
                </button>
              </li>
            );
          })}
        </ul>
      </nav>

      {visible.length === 0 ? (
        <p className="rounded-xl bg-zinc-50 px-3 py-10 text-center text-sm text-zinc-500 dark:bg-zinc-900">
          費目を選んでください
        </p>
      ) : (
        <>
          <svg
            viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
            className="w-full touch-pan-y select-none"
            role="img"
            aria-label={`歳出の推移（${mode === "per_capita" ? "1人あたり" : "総額"}）`}
          >
            {ticks.map((t) => (
              <g key={t}>
                <line
                  x1={PAD_LEFT}
                  x2={WIDTH - PAD_RIGHT}
                  y1={y(t)}
                  y2={y(t)}
                  strokeWidth={1}
                  className={
                    t === 0
                      ? "stroke-zinc-400 dark:stroke-zinc-600"
                      : "stroke-zinc-200 dark:stroke-zinc-800"
                  }
                />
                <text
                  x={PAD_LEFT - 4}
                  y={y(t) + 3}
                  textAnchor="end"
                  className="fill-zinc-500 text-[9px]"
                >
                  {tickLabel(t)}
                </text>
              </g>
            ))}
            {stacks.map((s, i) => {
              const x = PAD_LEFT + band * i + (band - barW) / 2;
              let acc = 0;
              return (
                <g
                  key={s.year}
                  opacity={i === shownIndex ? 1 : 0.55}
                  onClick={() => setTouchedYear(s.year)}
                  className="cursor-pointer"
                >
                  {/* 触れる範囲は棒より広い（年度の帯の全体）。選んでいる年度は薄い帯でハイライトする */}
                  <rect
                    x={PAD_LEFT + band * i}
                    y={PAD_TOP}
                    width={band}
                    height={innerH}
                    className={
                      i === shownIndex
                        ? "fill-zinc-200/70 dark:fill-zinc-700/50"
                        : "fill-transparent"
                    }
                  />
                  {s.parts.map((p) => {
                    const y0 = y(acc);
                    acc += p.value;
                    const h = y0 - y(acc);
                    return h - GAP > 0 ? (
                      <rect
                        key={p.key}
                        x={x}
                        y={y(acc) + GAP}
                        width={barW}
                        height={h - GAP}
                        // SVG の属性には CSS の変数が効かないので、色は style で指定する
                        style={{ fill: p.color }}
                      />
                    ) : null;
                  })}
                  <text
                    x={PAD_LEFT + band * i + band / 2}
                    y={HEIGHT - 6}
                    textAnchor="middle"
                    className={`text-[9px] ${
                      i === shownIndex
                        ? "fill-zinc-900 font-semibold dark:fill-zinc-100"
                        : "fill-zinc-500"
                    }`}
                  >
                    {shortYear(s.year)}
                  </text>
                </g>
              );
            })}
          </svg>

          <div
            className="mt-2 rounded-xl bg-zinc-50 p-3 text-xs dark:bg-zinc-900"
            aria-live="polite"
          >
            <p className="mb-1.5 flex justify-between text-sm">
              <span className="font-semibold">
                {fiscalYearLabel(shown.year)}
              </span>
              <span className="tabular-nums">
                {hidden.size > 0 ? "表示中の合計 " : "合計 "}
                <span className="font-semibold">
                  {format(totals[shownIndex])}
                </span>
              </span>
            </p>
            <ul className="flex flex-col gap-0.5">
              {[...shown.parts].reverse().map((p) => (
                <li
                  key={p.key}
                  className="flex items-center justify-between gap-2"
                >
                  <span className="flex items-center gap-1.5">
                    <span
                      aria-hidden
                      className="inline-block h-2 w-2 rounded-sm"
                      style={{ background: p.color }}
                    />
                    {p.key}
                  </span>
                  <span className="tabular-nums text-zinc-600 dark:text-zinc-400">
                    {format(p.value)}
                  </span>
                </li>
              ))}
            </ul>
            <p className="mt-1.5 text-zinc-500">
              棒に触れると、その年度の内訳が出ます。
            </p>
          </div>

          <details className="mt-2 text-xs text-zinc-600 dark:text-zinc-400">
            <summary className="cursor-pointer py-1">数値の一覧</summary>
            <table className="mt-1 w-full tabular-nums">
              <tbody>
                {stacks.map((s, i) => (
                  <tr
                    key={s.year}
                    className="border-t border-zinc-100 dark:border-zinc-800"
                  >
                    <td className="py-1">{fiscalYearLabel(s.year)}</td>
                    <td className="py-1 text-right">{format(totals[i])}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </details>
        </>
      )}
    </div>
  );
}
