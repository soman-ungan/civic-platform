// SPDX-License-Identifier: AGPL-3.0-only

import {
  RESIDENCY_ORDER,
  RESIDENCY_SHAPES,
  RESIDENCY_SHORT_LABELS,
  percent,
} from "@/lib/labels";
import type { ResidencyRow } from "@/lib/api/types/analysis";
import type { ResidencyType } from "@/lib/api/types/participation";

/** 表示する意見の数（差の大きい順） */
const MAX_ROWS = 5;

const WIDTH = 300;
const HEIGHT = 30;
const PAD = 12;
const AXIS_Y = 15;
const x = (rate: number) => PAD + rate * (WIDTH - PAD * 2);

/**
 * このまちとの関わり方による違い（ドット図）。意見ごとに 0〜100% の軸を引き、
 * 関わり方ごとの賛成率を点で置き、最大と最小を線で結ぶ。
 * 点は自由な位置で重なるため、4区分を色で区別しきれない（dataviz の検証）。色は使わず形で区別する。
 * 人数の少ない区分は backend で二次秘匿済み（null）。点を描かず「—」と出す
 */
export function ResidencyDotPlot({ rows }: { rows: ResidencyRow[] }) {
  const comparable = rows
    .map((row) => {
      const rates = RESIDENCY_ORDER.flatMap((t) => {
        const cell = row.by_type[t];
        return cell ? [{ type: t, rate: cell.agree / cell.seen, cell }] : [];
      });
      const values = rates.map((r) => r.rate);
      return {
        row,
        rates,
        spread:
          rates.length >= 2 ? Math.max(...values) - Math.min(...values) : -1,
      };
    })
    .filter((x) => x.spread >= 0)
    .sort((a, b) => b.spread - a.spread)
    .slice(0, MAX_ROWS);

  return (
    <div>
      <h3 className="mb-1 font-semibold">このまちとの関わり方による違い</h3>
      <p className="mb-2 text-xs leading-5 text-zinc-500">
        賛成した人の割合です。違いの大きい意見から{MAX_ROWS}
        件。5人未満の区分は表示しません（—）。
      </p>
      <ul className="mb-3 flex flex-wrap gap-x-3 gap-y-1 text-xs text-zinc-600 dark:text-zinc-400">
        {RESIDENCY_ORDER.map((t) => (
          <li key={t}>
            <span aria-hidden>{RESIDENCY_SHAPES[t]} </span>
            {RESIDENCY_SHORT_LABELS[t]}
          </li>
        ))}
      </ul>

      {comparable.length === 0 ? (
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          比べられるだけの人数が、まだ集まっていません。
        </p>
      ) : (
        <ul className="flex flex-col gap-4">
          {comparable.map(({ row, rates }) => {
            const values = rates.map((r) => r.rate);
            const [min, max] = [Math.min(...values), Math.max(...values)];
            return (
              <li key={row.statement_id}>
                <p className="mb-1 text-sm leading-6">{row.body}</p>
                <svg
                  viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
                  className="w-full text-zinc-800 dark:text-zinc-100"
                  aria-hidden
                >
                  {/* 軸（0 / 50 / 100%）は控えめに */}
                  <line
                    x1={x(0)}
                    x2={x(1)}
                    y1={AXIS_Y}
                    y2={AXIS_Y}
                    className="stroke-zinc-200 dark:stroke-zinc-700"
                    strokeWidth={2}
                    strokeLinecap="round"
                  />
                  {[0, 0.5, 1].map((t) => (
                    <line
                      key={t}
                      x1={x(t)}
                      x2={x(t)}
                      y1={AXIS_Y - 4}
                      y2={AXIS_Y + 4}
                      className="stroke-zinc-300 dark:stroke-zinc-600"
                      strokeWidth={1}
                    />
                  ))}
                  {/* 最大と最小を結ぶ線（違いの大きさ） */}
                  <line
                    x1={x(min)}
                    x2={x(max)}
                    y1={AXIS_Y}
                    y2={AXIS_Y}
                    className="stroke-zinc-500 dark:stroke-zinc-400"
                    strokeWidth={2}
                    strokeLinecap="round"
                  />
                  {rates.map(({ type, rate, cell }) => (
                    <Marker key={type} type={type} cx={x(rate)} cy={AXIS_Y}>
                      <title>{`${RESIDENCY_SHORT_LABELS[type]}: ${percent(cell.agree, cell.seen)}%（${cell.seen}人中${cell.agree}人）`}</title>
                    </Marker>
                  ))}
                </svg>
                <div className="flex justify-between px-[4%] text-[10px] text-zinc-400">
                  <span>0%</span>
                  <span>50%</span>
                  <span>100%</span>
                </div>
                {/* 値は文字でも示す（表の代わり） */}
                <p className="mt-1 text-xs text-zinc-600 dark:text-zinc-400">
                  {RESIDENCY_ORDER.map((t) => {
                    const cell = row.by_type[t];
                    return `${RESIDENCY_SHAPES[t]} ${RESIDENCY_SHORT_LABELS[t]} ${
                      cell ? `${percent(cell.agree, cell.seen)}%` : "—"
                    }`;
                  }).join(" ・ ")}
                </p>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

/** 関わり方ごとの形の点（直径 10px 前後、背景色の 2px の縁取りで重なっても見分けられる） */
function Marker({
  type,
  cx,
  cy,
  children,
}: {
  type: ResidencyType;
  cx: number;
  cy: number;
  children: React.ReactNode;
}) {
  const ring = {
    fill: "currentColor",
    stroke: "var(--background)",
    strokeWidth: 2,
    paintOrder: "stroke" as const,
  };
  const r = 5.5;
  switch (type) {
    case "resident":
      return (
        <circle cx={cx} cy={cy} r={r} {...ring}>
          {children}
        </circle>
      );
    case "neighbor":
      return (
        <rect x={cx - r} y={cy - r} width={r * 2} height={r * 2} {...ring}>
          {children}
        </rect>
      );
    case "commuter":
      return (
        <polygon
          points={`${cx},${cy - r - 1} ${cx + r + 1},${cy + r} ${cx - r - 1},${cy + r}`}
          {...ring}
        >
          {children}
        </polygon>
      );
    case "related":
      return (
        <polygon
          points={`${cx},${cy - r - 1} ${cx + r + 1},${cy} ${cx},${cy + r + 1} ${cx - r - 1},${cy}`}
          {...ring}
        >
          {children}
        </polygon>
      );
  }
}
