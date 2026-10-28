// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useRouter } from "next/navigation";
import { useRef } from "react";

import { tabsFor, transitionTo } from "@/lib/tabs";

/** スワイプとみなす横の移動（px）。縦の移動の2倍以上も必要（縦のスクロールと区別する） */
const MIN_DX = 60;
/** これより長く触れていたら、スワイプではない（ゆっくりした操作） */
const MAX_MS = 600;
/** 画面の端から始まる操作は、ブラウザの「戻る」に任せる */
const EDGE_PX = 24;

/**
 * タブのページの本体。左右のスワイプで隣のタブへ移る（タブは常に見えていて、スワイプは補助）。
 * 横にスクロールする所や、触れて操作する図には `data-swipe-ignore` を付け、そこでは反応しない
 */
export function SwipeNav({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const start = useRef<{ x: number; y: number; t: number } | null>(null);

  function onTouchStart(e: React.TouchEvent) {
    const touch = e.touches[0];
    const ignored =
      e.touches.length > 1 ||
      (e.target as Element).closest("[data-swipe-ignore]") !== null ||
      touch.clientX < EDGE_PX ||
      touch.clientX > window.innerWidth - EDGE_PX;
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
    // URL はフックで読まず、触れたときに読む（フックは Suspense の外で使えないため）
    const { tabs, current } = tabsFor(window.location.pathname);
    // 指を左へ動かしたら右のタブへ（ページをめくる向き）
    const next = current + (dx < 0 ? 1 : -1);
    if (
      current < 0 ||
      !tabs[current].swipe ||
      next < 0 ||
      next >= tabs.length
    ) {
      return;
    }
    router.push(tabs[next].href, {
      transitionTypes: transitionTo(current, next),
    });
  }

  return (
    <div
      className="flex min-h-0 flex-1 flex-col"
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      {children}
    </div>
  );
}
