// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useRef, useState } from "react";

/** スワイプとみなす横の移動（px）。縦の移動の2倍以上も必要（縦のスクロールと区別する） */
const MIN_DX = 50;
const MAX_MS = 600;

export type CarouselSection = {
  id: string;
  title: string;
  lead?: React.ReactNode;
  content: React.ReactNode;
};

/**
 * まちのお金の欄を、横フリックでリングのように切り替える（最後の次は最初に戻る）。
 * 見出しの左右の矢印と点でも切り替えられる（フリックに気づかない人のため）。
 * 欄の中身は全部描いたまま、見えない欄を隠す（推移の費目の選び方などを、切り替えても保つため）。
 * 中の横スクロール（`data-swipe-ignore`）の上で始めた操作では切り替えない
 */
export function FinanceCarousel({ sections }: { sections: CarouselSection[] }) {
  const [index, setIndex] = useState(0);
  const [direction, setDirection] = useState<"next" | "prev">("next");
  const root = useRef<HTMLDivElement>(null);
  const start = useRef<{ x: number; y: number; t: number } | null>(null);
  const n = sections.length;

  function go(step: 1 | -1) {
    setDirection(step > 0 ? "next" : "prev");
    setIndex((i) => (i + step + n) % n);
  }

  function onTouchStart(e: React.TouchEvent) {
    const touch = e.touches[0];
    // この部品の外枠も data-swipe-ignore（タブのスワイプに渡さないため）。それより内側の印だけを見る
    const ignored =
      e.touches.length > 1 ||
      (e.target as Element).closest("[data-swipe-ignore]") !== root.current;
    start.current = ignored
      ? null
      : { x: touch.clientX, y: touch.clientY, t: Date.now() };
  }

  function onTouchEnd(e: React.TouchEvent) {
    const s = start.current;
    start.current = null;
    if (!s) return;
    const touch = e.changedTouches[0];
    const dx = touch.clientX - s.x;
    const dy = touch.clientY - s.y;
    if (
      Math.abs(dx) < MIN_DX ||
      Math.abs(dx) < Math.abs(dy) * 2 ||
      Date.now() - s.t > MAX_MS
    ) {
      return;
    }
    // 指を左へ動かしたら次の欄へ（ページをめくる向き）
    go(dx < 0 ? 1 : -1);
  }

  const current = sections[index];
  return (
    <div
      ref={root}
      data-swipe-ignore
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
      className="min-h-full"
    >
      <div className="sticky top-0 z-10 -mx-4 mb-2 bg-[var(--background)] px-4 pt-1">
        <div className="flex items-center justify-between gap-2">
          <button
            type="button"
            aria-label={`前の欄（${sections[(index - 1 + n) % n].title}）`}
            onClick={() => go(-1)}
            className="flex h-11 w-11 items-center justify-center text-zinc-500"
          >
            ‹
          </button>
          <h2 className="font-semibold" aria-live="polite">
            {current.title}
          </h2>
          <button
            type="button"
            aria-label={`次の欄（${sections[(index + 1) % n].title}）`}
            onClick={() => go(1)}
            className="flex h-11 w-11 items-center justify-center text-zinc-500"
          >
            ›
          </button>
        </div>
        <div className="flex justify-center gap-1.5 pb-2" aria-hidden>
          {sections.map((s, i) => (
            <span
              key={s.id}
              className={`h-1.5 rounded-full transition-all ${
                i === index
                  ? "w-4 bg-zinc-800 dark:bg-zinc-200"
                  : "w-1.5 bg-zinc-300 dark:bg-zinc-700"
              }`}
            />
          ))}
        </div>
      </div>
      {sections.map((s, i) => (
        <section
          key={s.id}
          hidden={i !== index}
          aria-label={s.title}
          className={
            i === index
              ? direction === "next"
                ? "carousel-next"
                : "carousel-prev"
              : undefined
          }
        >
          {s.lead && (
            <p className="mb-3 text-xs leading-5 text-zinc-500">{s.lead}</p>
          )}
          {s.content}
        </section>
      ))}
    </div>
  );
}
