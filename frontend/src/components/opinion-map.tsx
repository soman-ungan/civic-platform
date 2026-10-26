// SPDX-License-Identifier: AGPL-3.0-only

import { GROUP_COLORS } from "@/lib/labels";
import type { GroupView, MapView } from "@/lib/types";

const SIZE = 300;
const PADDING = 24;

/**
 * 意見マップ。近い意見の参加者をまとめた点（基底クラスタ）を、人数に応じた大きさで描く。
 * 個人の位置は描かない（「あなた」だけは本人にのみ表示される）。
 */
export function OpinionMap({
  map,
  groups,
}: {
  map: MapView;
  groups: GroupView[];
}) {
  const colorOf = new Map(
    groups.map((g, i) => [g.id, GROUP_COLORS[i % GROUP_COLORS.length]]),
  );
  const xs = [
    ...map.base_clusters.map((c) => c.x),
    ...(map.you ? [map.you.x] : []),
  ];
  const ys = [
    ...map.base_clusters.map((c) => c.y),
    ...(map.you ? [map.you.y] : []),
  ];
  const [minX, maxX] = [Math.min(...xs), Math.max(...xs)];
  const [minY, maxY] = [Math.min(...ys), Math.max(...ys)];
  const span = Math.max(maxX - minX, maxY - minY, 1e-9);
  const scale = (SIZE - PADDING * 2) / span;
  // 中央に寄せ、y は上向きを正にする
  const sx = (x: number) =>
    PADDING +
    (x - minX) * scale +
    (SIZE - PADDING * 2 - (maxX - minX) * scale) / 2;
  const sy = (y: number) =>
    SIZE -
    (PADDING +
      (y - minY) * scale +
      (SIZE - PADDING * 2 - (maxY - minY) * scale) / 2);
  const maxCount = Math.max(...map.base_clusters.map((c) => c.count), 1);

  return (
    <figure>
      <svg
        viewBox={`0 0 ${SIZE} ${SIZE}`}
        role="img"
        aria-label="意見マップ。近い意見の人ほど近くに表示されます"
        className="aspect-square w-full rounded-2xl bg-zinc-50 dark:bg-zinc-900"
      >
        {map.base_clusters.map((c, i) => (
          <circle
            key={i}
            cx={sx(c.x)}
            cy={sy(c.y)}
            r={3 + 9 * Math.sqrt(c.count / maxCount)}
            fill={c.group_id === null ? "#a1a1aa" : colorOf.get(c.group_id)}
            fillOpacity={0.55}
          />
        ))}
        {map.you && (
          <g>
            <circle
              cx={sx(map.you.x)}
              cy={sy(map.you.y)}
              r={9}
              fill="none"
              stroke="currentColor"
              strokeWidth={2.5}
            />
            <text
              x={sx(map.you.x)}
              y={sy(map.you.y) - 14}
              textAnchor="middle"
              className="fill-current text-[11px] font-bold"
            >
              あなた
            </text>
          </g>
        )}
      </svg>
      <figcaption className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-zinc-600 dark:text-zinc-400">
        {groups.map((g, i) => (
          <span key={g.id} className="inline-flex items-center gap-1.5">
            <span
              aria-hidden
              className="inline-block size-2.5 rounded-full"
              style={{ backgroundColor: GROUP_COLORS[i % GROUP_COLORS.length] }}
            />
            グループ{g.label}（{g.size}人）
          </span>
        ))}
      </figcaption>
    </figure>
  );
}
