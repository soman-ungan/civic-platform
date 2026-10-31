// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect, useRef, useState } from "react";

import type { MapPoint, TopicMap } from "@/lib/api/types/digest";

import { MAP_HEIGHT } from "./use-shrinking-scale";

/** 地図の余白（座標は -1〜1） */
const PAD = 0.15;
/** 拡大の上限 */
const MAX_ZOOM = 8;
/** これより動いたら、タップではなくドラッグ（px） */
const TAP_PX = 6;

type View = { cx: number; cy: number; k: number };
const HOME: View = { cx: 0, cy: 0, k: 1 };

/**
 * 拡大・縮小と移動のできる地図（SVG）。ピンチ・ドラッグ・ホイールと「＋／−／元に戻す」のボタン。
 * 点と線の太さは、拡大しても画面の上で同じにする
 */
export function ZoomableMap({
  scale,
  map,
  anchor,
  near,
  selected,
  onPoint,
  onBackground,
}: {
  /** 大きさの倍率（1 で幅いっぱい・高さは画面の4割。縦横比を保って縮める） */
  scale: number;
  map: TopicMap;
  anchor: string | null;
  near: Set<string>;
  selected: string[];
  onPoint: (id: string) => void;
  onBackground: () => void;
}) {
  const svgRef = useRef<SVGSVGElement>(null);
  const outerRef = useRef<HTMLDivElement>(null);
  /** 縮める前の枠の大きさ（px）。縮めたときの大きさは、これに倍率を掛けてその場で出す
   * （縮んだ枠を測り直すと1コマ遅れて、点やボタンが揺れるため） */
  const [full, setFull] = useState({ w: 1, h: 1 });
  const [view, setView] = useState<View>(HOME);
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef({ moved: 0, downId: null as string | null });

  useEffect(() => {
    const el = outerRef.current;
    if (!el) return;
    const measure = () =>
      setFull({
        w: el.clientWidth,
        h: window.innerHeight * MAP_HEIGHT.large,
      });
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);
  const size = { w: full.w * scale, h: full.h * scale };

  // 地図の座標で見える範囲（縦横比は枠に合わせる。-1〜1 と余白が必ず入る。縮めても縦横比は同じ）
  const aspect = full.w / Math.max(full.h, 1);
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
  // 起点の近傍を、起点を中心にした薄い円で示す（半径は、いちばん遠い近傍の点まで＋少しの余白）。
  // 地図の距離は似ている度合いをおおまかに表すだけなので、円の中に近傍でない点が入ることもある（近傍は線と色で見分ける）
  const anchorPoint = anchor ? byId.get(anchor) : undefined;
  const halo = anchorPoint && {
    x: anchorPoint.x,
    y: anchorPoint.y,
    r:
      Math.max(
        0.1,
        ...[...near].flatMap((id) => {
          const q = byId.get(id);
          return q
            ? [Math.hypot(q.x - anchorPoint.x, q.y - anchorPoint.y)]
            : [];
        }),
      ) +
      14 * unit,
  };
  const radiusPx = (p: MapPoint) =>
    4 + 5 * Math.sqrt(p.participants / maxParticipants);

  return (
    <div ref={outerRef} className="w-full">
      <div className="relative mx-auto" style={{ width: `${scale * 100}%` }}>
        <svg
          ref={svgRef}
          viewBox={`${view.cx - hx} ${view.cy - hy} ${2 * hx} ${2 * hy}`}
          style={{ height: `calc(${MAP_HEIGHT.large * 100}dvh * ${scale})` }}
          className="w-full touch-none rounded-2xl bg-zinc-50 select-none dark:bg-zinc-900"
          role="group"
          aria-label="締め切ったトピックの地図（似ているトピックほど近い）"
          data-swipe-ignore
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
        >
          {halo && (
            <circle
              cx={halo.x}
              cy={-halo.y}
              r={halo.r}
              strokeWidth={unit}
              className="fill-amber-300/20 stroke-amber-400/60 dark:fill-amber-500/15"
            />
          )}
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
    </div>
  );
}
