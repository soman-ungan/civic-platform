// SPDX-License-Identifier: AGPL-3.0-only

import {
  GROUP_COLORS,
  RESIDENCY_COLORS,
  RESIDENCY_ORDER,
  RESIDENCY_SHORT_LABELS,
  percent,
} from "@/lib/labels";
import type { GroupComposition, GroupView } from "@/lib/types";

/**
 * 意見グループごとの、このまちとの関わり方の内訳（100% 積み上げの帯）。
 * Polis のレポートの「各グループの内訳」と同じく、グループを説明するためだけに使う。
 * 人数の少ない区分は backend で二次秘匿済み。隠れた分は灰色の「5人未満の区分」にまとめる。
 * 本人の属性や地区では分けない（docs/adr/0002-participant-attributes.md）
 */
export function GroupCompositionChart({
  groups,
  composition,
}: {
  groups: GroupView[];
  composition: GroupComposition[];
}) {
  const byGroup = new Map(composition.map((c) => [c.group_id, c.by_type]));

  return (
    <div>
      <h3 className="mb-1 font-semibold">
        グループの構成（このまちとの関わり方）
      </h3>
      <p className="mb-3 text-xs leading-5 text-zinc-500">
        どのグループにも、いろいろな関わり方の人がいます。参加者の中での内訳です（無作為の標本ではありません）。
      </p>

      {/* 凡例（2区分以上なので常に表示する） */}
      <ul className="mb-3 flex flex-wrap gap-x-3 gap-y-1 text-xs text-zinc-600 dark:text-zinc-400">
        {RESIDENCY_ORDER.map((t) => (
          <li key={t} className="inline-flex items-center gap-1">
            <span
              aria-hidden
              className="inline-block size-3 rounded-sm"
              style={{ backgroundColor: RESIDENCY_COLORS[t] }}
            />
            {RESIDENCY_SHORT_LABELS[t]}
          </li>
        ))}
        <li className="inline-flex items-center gap-1">
          <span
            aria-hidden
            className="inline-block size-3 rounded-sm"
            style={{ backgroundColor: "var(--residency-hidden)" }}
          />
          5人未満の区分（まとめて表示）
        </li>
      </ul>

      <ul className="flex flex-col gap-4">
        {groups.map((g, i) => {
          const byType = byGroup.get(g.id);
          const visible = RESIDENCY_ORDER.flatMap((t) => {
            const n = byType?.[t];
            return n ? [{ type: t, n }] : [];
          });
          const hidden = Math.max(
            0,
            g.size - visible.reduce((sum, v) => sum + v.n, 0),
          );
          const segments = [
            ...visible.map((v) => ({
              key: v.type,
              n: v.n,
              color: RESIDENCY_COLORS[v.type],
              label: RESIDENCY_SHORT_LABELS[v.type],
            })),
            ...(hidden > 0
              ? [
                  {
                    key: "hidden",
                    n: hidden,
                    color: "var(--residency-hidden)",
                    label: "5人未満の区分",
                  },
                ]
              : []),
          ];
          return (
            <li key={g.id}>
              <p className="mb-1 flex items-center gap-1.5 text-sm font-semibold">
                <span
                  aria-hidden
                  className="inline-block size-2.5 rounded-full"
                  style={{
                    backgroundColor: GROUP_COLORS[i % GROUP_COLORS.length],
                  }}
                />
                グループ{g.label}
                <span className="font-normal text-zinc-500">
                  （{g.size}人）
                </span>
              </p>
              {/* 100% 積み上げの帯。区分の間は 2px のすき間で区切る */}
              <div
                aria-hidden
                className="flex h-3 w-full gap-[2px] overflow-hidden rounded"
              >
                {segments.map((s) => (
                  <div
                    key={s.key}
                    className="h-full first:rounded-l last:rounded-r"
                    style={{
                      width: `${(s.n / g.size) * 100}%`,
                      backgroundColor: s.color,
                    }}
                    title={`${s.label}: ${percent(s.n, g.size)}%（${s.n}人）`}
                  />
                ))}
              </div>
              {/* 値は文字でも示す（色だけに頼らない） */}
              <p className="mt-1 text-xs text-zinc-600 dark:text-zinc-400">
                {visible.length === 0
                  ? "人数が少ないため、内訳は表示しません。"
                  : segments
                      .map((s) => `${s.label} ${percent(s.n, g.size)}%`)
                      .join(" ・ ")}
              </p>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
