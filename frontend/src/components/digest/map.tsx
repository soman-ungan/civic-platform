// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useRef, useState } from "react";

import { BUTTON_PRIMARY } from "@/components/ui/styles";
import type { MapPoint, TopicMap } from "@/lib/api/types/digest";
import { MAP_ANCHOR, digestHref } from "@/lib/digest";

import { PointPopup, TopicCard } from "./map-cards";
import { useShrinkingScale } from "./use-shrinking-scale";
import { ZoomableMap } from "./zoomable-map";

/** まとめで選べるトピックの数（backend の digest::MAX_TOPICS） */
const MAX_TOPICS = 10;

/**
 * トピックマップ: 締め切ったトピック1件を点1つにし、似ているトピックほど近くに置く（配置は backend の PCA）。
 * 見出しと地図はページの上に固定し、その下のカードだけがスクロールする。
 * 点を押すと概要の吹き出し（押すとトピック詳細のレイヤ。「ここを起点にする」で起点に）。
 * 起点を選ぶと近傍（線でつながった、似ているトピック）を色付け、カードで近傍を選んで「まとめを見る」へ
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
  const onMap = (id: string | null) =>
    id !== null && map.points.some((p) => p.id === id);
  const [anchor, setAnchor] = useState<string | null>(
    onMap(initialAnchor) ? initialAnchor : null,
  );
  const [selected, setSelected] = useState<string[]>([]);
  const [popup, setPopup] = useState<string | null>(
    onMap(initialAnchor) ? initialAnchor : null,
  );
  const byId = new Map(map.points.map((p) => [p.id, p]));
  const tag = map.tags.find((t) => t.id === map.tag);
  // 起点の近傍（似ている順）
  const neighbors = anchor
    ? map.edges
        .flatMap((e) =>
          e.a === anchor
            ? [{ id: e.b, score: e.score }]
            : e.b === anchor
              ? [{ id: e.a, score: e.score }]
              : [],
        )
        .sort((a, b) => b.score - a.score)
    : [];
  const near = new Set(neighbors.map((n) => n.id));

  function chooseAnchor(id: string | null) {
    setAnchor(id);
    setSelected([]);
  }
  function toggle(id: string) {
    setSelected((s) =>
      s.includes(id)
        ? s.filter((x) => x !== id)
        : s.length + 1 < MAX_TOPICS
          ? [...s, id]
          : s,
    );
  }

  const cards: MapPoint[] = anchor
    ? [anchor, ...neighbors.map((n) => n.id)].flatMap(
        (id) => byId.get(id) ?? [],
      )
    : [...map.points].sort((a, b) => b.closes_at.localeCompare(a.closes_at));
  const popupPoint = popup ? byId.get(popup) : undefined;
  const sentinel = useRef<HTMLDivElement>(null);
  const mapScale = useShrinkingScale(sentinel);
  // 地図が縮んだら、吹き出しは閉じる（小さい地図を覆ってしまうため）
  const shrunk = mapScale < 0.9;

  return (
    <>
      <div ref={sentinel} aria-hidden />
      {/* 見出しと地図は、ページをスクロールしても上に残す。カードをスクロールすると地図は縮む */}
      <div className="sticky top-0 z-10 -mx-4 bg-white px-4 pt-2 pb-2 dark:bg-zinc-950">
        <div className="flex items-baseline justify-between gap-2">
          <h2 className="text-base font-bold">
            これまでの議論
            {tag && (
              <span className="ml-2 text-sm font-normal text-zinc-500">
                {tag.kind === "district" ? `#${tag.name}` : tag.name}
              </span>
            )}
          </h2>
          <Link
            href={`/${slug}/consensus#${MAP_ANCHOR}`}
            className="shrink-0 text-xs text-zinc-600 underline underline-offset-2 dark:text-zinc-400"
          >
            ← 分野を選び直す
          </Link>
        </div>
        <div className="relative mt-2">
          <ZoomableMap
            scale={mapScale}
            map={map}
            anchor={anchor}
            near={near}
            selected={selected}
            onPoint={(id) => setPopup(id)}
            onBackground={() => setPopup(null)}
          />
          {popupPoint && !shrunk && (
            <PointPopup
              slug={slug}
              point={popupPoint}
              isAnchor={popupPoint.id === anchor}
              onAnchor={() => {
                chooseAnchor(popupPoint.id === anchor ? null : popupPoint.id);
                setPopup(null);
              }}
              onClose={() => setPopup(null)}
            />
          )}
        </div>
      </div>

      <p className="mt-2 text-xs leading-5 text-zinc-500">
        {anchor
          ? "起点と、起点に似ているトピック（黄色）です。カードを押すと、まとめに入れます。"
          : "点は締め切ったトピックで、大きいほど参加した人が多く、似ているトピックほど近くにあります。カードを押すと、そのトピックを起点にします。"}
      </p>
      {/* 下の余白は、地図が縮む分（縮んでページが短くなり、スクロールが押し戻されて地図が揺れないように） */}
      <ul className="mt-2 flex flex-col gap-2 pb-[22dvh]">
        {cards.map((p) => (
          <TopicCard
            key={p.id}
            slug={slug}
            point={p}
            role={
              p.id === anchor
                ? "anchor"
                : selected.includes(p.id)
                  ? "selected"
                  : anchor
                    ? "neighbor"
                    : "plain"
            }
            onPress={() =>
              p.id === anchor
                ? undefined
                : anchor
                  ? toggle(p.id)
                  : chooseAnchor(p.id)
            }
            onClearAnchor={() => chooseAnchor(null)}
          />
        ))}
        {anchor && neighbors.length === 0 && (
          <li className="text-xs text-zinc-500">
            この分野には、起点に似ている締め切ったトピックがありません。1件だけでもまとめを見られます。
          </li>
        )}
      </ul>

      {anchor && (
        <div className="sticky bottom-0 z-10 -mx-4 border-t border-zinc-200 bg-white/95 px-4 py-3 backdrop-blur dark:border-zinc-800 dark:bg-zinc-950/95">
          <Link
            href={digestHref(slug, anchor, selected)}
            className={`flex h-11 items-center justify-center text-sm ${BUTTON_PRIMARY}`}
          >
            まとめを見る（{1 + selected.length}件）
          </Link>
        </div>
      )}
    </>
  );
}
