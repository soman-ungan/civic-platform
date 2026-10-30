// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import { ChipLink } from "@/components/ui/chip";
import { EmptyState } from "@/components/ui/empty-state";
import { ScrollRow } from "@/components/ui/scroll-row";
import { BUTTON_PRIMARY } from "@/components/ui/styles";
import type { MapPoint, TopicMap } from "@/lib/api/types/digest";
import { consensusMapHref, digestHref, formatDate } from "@/lib/digest";

/** まとめで選べるトピックの数（backend の digest::MAX_TOPICS） */
const MAX_TOPICS = 10;
/** 地図の余白（座標は -1〜1） */
const PAD = 0.15;

/**
 * トピックマップ: 締め切ったトピック1件を点1つにし、似ているトピックほど近くに置く（配置は backend の PCA）。
 * 点を押すと起点になり、起点に似ているトピック（近傍。線でつながったもの）だけを追加で選べる
 */
export function TopicMapView({
  slug,
  map,
  initialAnchor = null,
}: {
  slug: string;
  map: TopicMap;
  /** 最初から起点にするトピック（合意点から来たとき。地図に載っていなければ無視） */
  initialAnchor?: string | null;
}) {
  const [anchor, setAnchor] = useState<string | null>(
    initialAnchor && map.points.some((p) => p.id === initialAnchor)
      ? initialAnchor
      : null,
  );
  const [selected, setSelected] = useState<string[]>([]);
  const byId = new Map(map.points.map((p) => [p.id, p]));
  const neighbors = new Set(
    anchor
      ? map.edges.flatMap((e) =>
          e.a === anchor ? [e.b] : e.b === anchor ? [e.a] : [],
        )
      : [],
  );

  function press(id: string) {
    if (id === anchor) {
      setAnchor(null);
      setSelected([]);
    } else if (anchor && neighbors.has(id)) {
      setSelected((s) =>
        s.includes(id)
          ? s.filter((x) => x !== id)
          : s.length + 1 < MAX_TOPICS
            ? [...s, id]
            : s,
      );
    } else {
      setAnchor(id);
      setSelected([]);
    }
  }

  const maxParticipants = Math.max(1, ...map.points.map((p) => p.participants));
  const radius = (p: MapPoint) =>
    0.035 + 0.045 * Math.sqrt(p.participants / maxParticipants);
  const listed: MapPoint[] = anchor
    ? [anchor, ...neighbors].flatMap((id) => byId.get(id) ?? [])
    : map.points;

  return (
    <div className="flex flex-col gap-3 pb-4">
      <ScrollRow label="分野・地区">
        {map.tags.map((t) => (
          <li key={t.id}>
            <ChipLink
              active={t.id === map.tag}
              href={consensusMapHref(slug, { tag: t.id })}
            >
              {t.kind === "district" ? `#${t.name}` : t.name}（{t.count}）
            </ChipLink>
          </li>
        ))}
      </ScrollRow>

      {map.points.length === 0 ? (
        <EmptyState>まだ締め切ったトピックはありません</EmptyState>
      ) : (
        <>
          <svg
            viewBox={`${-1 - PAD} ${-1 - PAD} ${2 + 2 * PAD} ${2 + 2 * PAD}`}
            className="aspect-square w-full rounded-2xl bg-zinc-50 dark:bg-zinc-900"
            role="group"
            aria-label="締め切ったトピックの地図（似ているトピックほど近い）"
            data-swipe-ignore
          >
            {map.edges.map((e) => {
              const a = byId.get(e.a);
              const b = byId.get(e.b);
              if (!a || !b) return null;
              const lit = anchor !== null && (e.a === anchor || e.b === anchor);
              return (
                <line
                  key={`${e.a}-${e.b}`}
                  x1={a.x}
                  y1={-a.y}
                  x2={b.x}
                  y2={-b.y}
                  strokeWidth={lit ? 0.012 : 0.006}
                  className={
                    lit
                      ? "stroke-amber-500"
                      : "stroke-zinc-300 dark:stroke-zinc-700"
                  }
                />
              );
            })}
            {map.points.map((p) => {
              const role =
                p.id === anchor
                  ? "fill-zinc-900 dark:fill-zinc-100"
                  : selected.includes(p.id)
                    ? "fill-sky-600"
                    : neighbors.has(p.id)
                      ? "fill-amber-400"
                      : anchor
                        ? "fill-zinc-300 dark:fill-zinc-700"
                        : "fill-sky-700/70 dark:fill-sky-400/70";
              return (
                <circle
                  key={p.id}
                  cx={p.x}
                  cy={-p.y}
                  r={radius(p)}
                  className={`cursor-pointer ${role}`}
                  onClick={() => press(p.id)}
                >
                  <title>{p.title}</title>
                </circle>
              );
            })}
          </svg>
          <p className="text-xs leading-5 text-zinc-500">
            点は締め切ったトピックで、大きいほど参加した人が多く、似ているトピックほど近くにあります。点を押すと起点になり、似ているトピック（黄色）を足してまとめを見られます。
          </p>

          <ul className="flex flex-col gap-2">
            {listed.map((p) => {
              const isAnchor = p.id === anchor;
              const isSelected = selected.includes(p.id);
              return (
                <li key={p.id}>
                  <button
                    type="button"
                    onClick={() => press(p.id)}
                    aria-pressed={isAnchor || isSelected}
                    className={`block w-full rounded-2xl border p-3 text-left ${
                      isAnchor
                        ? "border-zinc-900 dark:border-zinc-100"
                        : isSelected
                          ? "border-sky-600"
                          : "border-zinc-200 dark:border-zinc-800"
                    }`}
                  >
                    <span className="text-xs text-zinc-500">
                      {isAnchor
                        ? "起点"
                        : isSelected
                          ? "選んだ"
                          : anchor
                            ? "似ているトピック（押すと足す）"
                            : `${formatDate(p.closes_at)}に締め切り`}
                      {" ・ "}参加 {p.participants} 人
                    </span>
                    <span className="mt-0.5 block text-sm leading-snug font-medium">
                      {p.title}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
          {anchor && neighbors.size === 0 && (
            <p className="text-xs text-zinc-500">
              この分野には、起点に似ている締め切ったトピックがありません。1件だけでもまとめを見られます。
            </p>
          )}

          {anchor && (
            <div className="sticky bottom-0 -mx-4 border-t border-zinc-200 bg-white/95 px-4 py-3 backdrop-blur dark:border-zinc-800 dark:bg-zinc-950/95">
              <Link
                href={digestHref(slug, anchor, selected)}
                className={`flex h-11 items-center justify-center text-sm ${BUTTON_PRIMARY}`}
              >
                まとめを見る（{1 + selected.length}件）
              </Link>
            </div>
          )}
        </>
      )}
    </div>
  );
}
