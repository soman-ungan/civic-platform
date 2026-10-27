// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

export type DonutSlice = {
  key: string;
  label: string;
  /** 割合の計算に使う値（どの単位でもよい） */
  value: number;
  /** 区切りに触れたとき、真ん中に出す額 */
  valueText: string;
  shareText: string;
  /** 系列の色（`var(--series-n)`） */
  color: string;
};

const SIZE = 200;
const R = 78;
const WIDTH = 30;
/** 区切りの間のすき間（円周上の px） */
const GAP = 2;
const C = 2 * Math.PI * R;

/**
 * ドーナツ型の円グラフ（全体の中の割合をひと目で見せる。区切りは6つまで: dataviz）。
 * 近い値は円では比べにくいので、呼び出し側で色見本付きの一覧（凡例を兼ねる）を下に並べる。
 * 区切りに触れると、その名前・額・割合を真ん中に出す。もう一度触れると合計に戻る
 */
export function FinanceDonut({
  slices,
  totalLabel,
  totalText,
}: {
  slices: DonutSlice[];
  totalLabel: string;
  totalText: string;
}) {
  const [active, setActive] = useState<string | null>(null);
  const total = slices.reduce((a, s) => a + s.value, 0);
  if (total <= 0) return null;
  const shown = slices.find((s) => s.key === active);
  // 区切りごとの円周上の長さと、始まる位置
  const arcs = slices.map((s, i) => ({
    slice: s,
    len: (s.value / total) * C,
    start: slices.slice(0, i).reduce((a, p) => a + (p.value / total) * C, 0),
  }));

  return (
    <div className="relative mx-auto mb-4 w-48" data-swipe-ignore>
      <svg
        viewBox={`0 0 ${SIZE} ${SIZE}`}
        className="w-full -rotate-90"
        role="img"
        aria-label={slices.map((s) => `${s.label} ${s.shareText}`).join("、")}
      >
        {arcs.map(({ slice: s, len, start }) => {
          const dash = Math.max(len - GAP, 0.5);
          return (
            <circle
              key={s.key}
              cx={SIZE / 2}
              cy={SIZE / 2}
              r={R}
              fill="none"
              strokeWidth={WIDTH}
              strokeDasharray={`${dash} ${C - dash}`}
              strokeDashoffset={-start}
              // SVG の属性には CSS の変数が効かないので、色は style で指定する
              style={{
                stroke: s.color,
                opacity: active && active !== s.key ? 0.35 : 1,
              }}
              className="cursor-pointer transition-opacity"
              onClick={() => setActive(active === s.key ? null : s.key)}
            />
          );
        })}
      </svg>
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
        <span className="text-xs text-zinc-500">
          {shown ? shown.label : totalLabel}
        </span>
        <span className="text-lg font-bold tabular-nums">
          {shown ? shown.valueText : totalText}
        </span>
        {shown && (
          <span className="text-xs tabular-nums text-zinc-500">
            {shown.shareText}
          </span>
        )}
      </div>
    </div>
  );
}
