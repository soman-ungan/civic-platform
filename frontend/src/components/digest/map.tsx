// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";

import { BUTTON_PRIMARY, BUTTON_SECONDARY } from "@/components/ui/styles";
import type { MapPoint, MapTag, TopicMap } from "@/lib/api/types/digest";
import {
  MAP_ANCHOR,
  consensusMapHref,
  digestHref,
  formatDate,
} from "@/lib/digest";

/** まとめで選べるトピックの数（backend の digest::MAX_TOPICS） */
const MAX_TOPICS = 10;
/** 地図の余白（座標は -1〜1） */
const PAD = 0.15;
/** 拡大の上限 */
const MAX_ZOOM = 8;
/** これより動いたら、タップではなくドラッグ（px） */
const TAP_PX = 6;

/** 地図を開く前のタグの一覧 */
export function MapTagList({ slug, tags }: { slug: string; tags: MapTag[] }) {
  if (tags.length === 0) {
    return (
      <p className="rounded-xl bg-zinc-50 px-4 py-10 text-center text-sm text-zinc-500 dark:bg-zinc-900">
        まだ締め切ったトピックはありません
      </p>
    );
  }
  return (
    <ul className="grid grid-cols-2 gap-2">
      {tags.map((t) => (
        <li key={t.id}>
          <Link
            href={consensusMapHref(slug, { tag: t.id })}
            className="flex h-full flex-col rounded-2xl border border-zinc-200 p-3 dark:border-zinc-800"
          >
            <span className="text-sm font-medium">
              {t.kind === "district" ? `#${t.name}` : t.name}
            </span>
            <span className="mt-0.5 text-xs text-zinc-500">
              締め切ったトピック {t.count} 件
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

type View = { cx: number; cy: number; k: number };
const HOME: View = { cx: 0, cy: 0, k: 1 };

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

  return (
    <>
      {/* 見出しと地図は、ページをスクロールしても上に残す */}
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
            map={map}
            anchor={anchor}
            near={near}
            selected={selected}
            onPoint={(id) => setPopup(id)}
            onBackground={() => setPopup(null)}
          />
          {popupPoint && (
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
      <ul className="mt-2 flex flex-col gap-2 pb-4">
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

/**
 * 拡大・縮小と移動のできる地図（SVG）。ピンチ・ドラッグ・ホイールと「＋／−／元に戻す」のボタン。
 * 点と線の太さは、拡大しても画面の上で同じにする
 */
function ZoomableMap({
  map,
  anchor,
  near,
  selected,
  onPoint,
  onBackground,
}: {
  map: TopicMap;
  anchor: string | null;
  near: Set<string>;
  selected: string[];
  onPoint: (id: string) => void;
  onBackground: () => void;
}) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [size, setSize] = useState({ w: 1, h: 1 });
  const [view, setView] = useState<View>(HOME);
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef({ moved: 0, downId: null as string | null });

  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({ w: entry.contentRect.width, h: entry.contentRect.height }),
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // 地図の座標で見える範囲（縦横比は箱に合わせる。-1〜1 と余白が必ず入る）
  const aspect = size.w / Math.max(size.h, 1);
  const base = 1 + PAD;
  const hx = (aspect >= 1 ? base * aspect : base) / view.k;
  const hy = (aspect >= 1 ? base : base / aspect) / view.k;
  /** 画面の 1px が地図の何単位か */
  const unit = (2 * hx) / Math.max(size.w, 1);

  const clampZoom = (k: number) => Math.min(MAX_ZOOM, Math.max(1, k));
  function zoomAt(factor: number, px?: number, py?: number) {
    setView((v) => {
      const k = clampZoom(v.k * factor);
      if (px === undefined || py === undefined) return { ...v, k };
      // 指の位置（画面の px）を地図の座標にして、そこが動かないように中心をずらす
      const mx = v.cx + (px - size.w / 2) * unit;
      const my = v.cy + (py - size.h / 2) * unit;
      const r = v.k / k;
      return { k, cx: mx - (mx - v.cx) * r, cy: my - (my - v.cy) * r };
    });
  }

  // ホイールでの拡大縮小（ページのスクロールを止めるため passive にしない）
  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      zoomAt(
        e.deltaY < 0 ? 1.2 : 1 / 1.2,
        e.clientX - rect.left,
        e.clientY - rect.top,
      );
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  function local(e: React.PointerEvent) {
    const rect = svgRef.current!.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }
  function onPointerDown(e: React.PointerEvent<SVGSVGElement>) {
    const id = (e.target as Element).getAttribute("data-id");
    if (pointers.current.size === 0) gesture.current = { moved: 0, downId: id };
    pointers.current.set(e.pointerId, local(e));
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: React.PointerEvent<SVGSVGElement>) {
    const prev = pointers.current.get(e.pointerId);
    if (!prev) return;
    const next = local(e);
    const others = [...pointers.current.entries()].filter(
      ([pid]) => pid !== e.pointerId,
    );
    if (others.length === 0) {
      // 1本の指: 動かす
      const dx = next.x - prev.x;
      const dy = next.y - prev.y;
      gesture.current.moved += Math.abs(dx) + Math.abs(dy);
      setView((v) => ({ ...v, cx: v.cx - dx * unit, cy: v.cy - dy * unit }));
    } else {
      // 2本の指: 指の間の距離の比で拡大縮小
      const other = others[0][1];
      const before = Math.hypot(prev.x - other.x, prev.y - other.y);
      const after = Math.hypot(next.x - other.x, next.y - other.y);
      gesture.current.moved += TAP_PX;
      if (before > 0) {
        zoomAt(after / before, (next.x + other.x) / 2, (next.y + other.y) / 2);
      }
    }
    pointers.current.set(e.pointerId, next);
  }
  function onPointerUp(e: React.PointerEvent<SVGSVGElement>) {
    pointers.current.delete(e.pointerId);
    if (pointers.current.size > 0) return;
    if (gesture.current.moved < TAP_PX) {
      const id = gesture.current.downId;
      if (id) onPoint(id);
      else onBackground();
    }
  }

  const byId = new Map(map.points.map((p) => [p.id, p]));
  const maxParticipants = Math.max(1, ...map.points.map((p) => p.participants));
  const radiusPx = (p: MapPoint) =>
    7 + 9 * Math.sqrt(p.participants / maxParticipants);

  return (
    <div className="relative">
      <svg
        ref={svgRef}
        viewBox={`${view.cx - hx} ${view.cy - hy} ${2 * hx} ${2 * hy}`}
        className="h-[40dvh] w-full touch-none rounded-2xl bg-zinc-50 select-none dark:bg-zinc-900"
        role="group"
        aria-label="締め切ったトピックの地図（似ているトピックほど近い）"
        data-swipe-ignore
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
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
              strokeWidth={(lit ? 2.5 : 1.2) * unit}
              className={
                lit
                  ? "stroke-amber-500"
                  : "stroke-zinc-300 dark:stroke-zinc-700"
              }
            />
          );
        })}
        {map.points.map((p) => {
          const fill =
            p.id === anchor
              ? "fill-zinc-900 dark:fill-zinc-100"
              : selected.includes(p.id)
                ? "fill-sky-600"
                : near.has(p.id)
                  ? "fill-amber-400"
                  : anchor
                    ? "fill-zinc-300 dark:fill-zinc-700"
                    : "fill-sky-700/70 dark:fill-sky-400/70";
          return (
            <circle
              key={p.id}
              data-id={p.id}
              cx={p.x}
              cy={-p.y}
              r={radiusPx(p) * unit}
              className={`cursor-pointer ${fill}`}
            >
              <title>{p.title}</title>
            </circle>
          );
        })}
      </svg>
      <div className="absolute top-2 right-2 flex flex-col gap-1">
        {[
          { label: "拡大", text: "＋", act: () => zoomAt(1.5) },
          { label: "縮小", text: "−", act: () => zoomAt(1 / 1.5) },
          { label: "元の大きさに戻す", text: "⟲", act: () => setView(HOME) },
        ].map((b) => (
          <button
            key={b.label}
            type="button"
            aria-label={b.label}
            onClick={b.act}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-white/90 text-sm shadow dark:bg-zinc-800/90"
          >
            {b.text}
          </button>
        ))}
      </div>
    </div>
  );
}

/** 点を押したときの概要。概要を押すとトピック詳細（合意点のページの上に重ねるレイヤ） */
function PointPopup({
  slug,
  point,
  isAnchor,
  onAnchor,
  onClose,
}: {
  slug: string;
  point: MapPoint;
  isAnchor: boolean;
  onAnchor: () => void;
  onClose: () => void;
}) {
  return (
    <div className="absolute inset-x-2 bottom-2 rounded-2xl border border-zinc-200 bg-white p-3 shadow-lg dark:border-zinc-700 dark:bg-zinc-900">
      <div className="flex items-start gap-2">
        <Link href={`/${slug}/topics/${point.id}`} className="min-w-0 flex-1">
          <span className="block text-sm leading-snug font-medium underline-offset-2 hover:underline">
            {point.title}
          </span>
          <span className="mt-0.5 block text-xs text-zinc-500">
            {formatDate(point.closes_at)}に締め切り ・ 参加 {point.participants}{" "}
            人 ・ 押すと詳細
          </span>
        </Link>
        <button
          type="button"
          aria-label="閉じる"
          onClick={onClose}
          className="h-6 w-6 shrink-0 text-zinc-500"
        >
          ×
        </button>
      </div>
      <button
        type="button"
        onClick={onAnchor}
        className={`mt-2 h-8 w-full text-xs ${BUTTON_SECONDARY}`}
      >
        {isAnchor ? "起点を外す" : "ここを起点にする"}
      </button>
    </div>
  );
}

function TopicCard({
  slug,
  point,
  role,
  onPress,
  onClearAnchor,
}: {
  slug: string;
  point: MapPoint;
  role: "anchor" | "selected" | "neighbor" | "plain";
  onPress: () => void;
  onClearAnchor: () => void;
}) {
  const label = {
    anchor: "起点",
    selected: "まとめに入れた",
    neighbor: "似ているトピック（押すとまとめに入れる）",
    plain: `${formatDate(point.closes_at)}に締め切り`,
  }[role];
  const border = {
    anchor: "border-zinc-900 dark:border-zinc-100",
    selected: "border-sky-600 bg-sky-50 dark:bg-sky-950/40",
    neighbor: "border-amber-400",
    plain: "border-zinc-200 dark:border-zinc-800",
  }[role];
  return (
    <li className={`flex items-stretch rounded-2xl border ${border}`}>
      <button
        type="button"
        onClick={onPress}
        aria-pressed={role === "anchor" || role === "selected"}
        className="min-w-0 flex-1 p-3 text-left"
      >
        <span className="text-xs text-zinc-500">
          {label} ・ 参加 {point.participants} 人
        </span>
        <span className="mt-0.5 block text-sm leading-snug font-medium">
          {point.title}
        </span>
      </button>
      <div className="flex shrink-0 flex-col items-end justify-center gap-1 pr-3">
        <Link
          href={`/${slug}/topics/${point.id}`}
          className="text-xs text-zinc-600 underline underline-offset-2 dark:text-zinc-400"
        >
          詳細
        </Link>
        {role === "anchor" && (
          <button
            type="button"
            onClick={onClearAnchor}
            className="text-xs text-zinc-500"
          >
            外す
          </button>
        )}
      </div>
    </li>
  );
}
