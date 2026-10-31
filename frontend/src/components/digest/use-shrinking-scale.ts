// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect, useState } from "react";

/** 地図の高さ（画面の高さに対する割合）。カードをスクロールすると、縦横比を保ったまま大きいほうから小さいほうへ縮む */
export const MAP_HEIGHT = { large: 0.4, small: 0.18 } as const;
/** いちばん近い、縦にスクロールする祖先 */
function scrollParent(el: HTMLElement | null): HTMLElement | null {
  for (let p = el?.parentElement; p; p = p.parentElement) {
    const { overflowY } = getComputedStyle(p);
    if (overflowY === "auto" || overflowY === "scroll") return p;
  }
  return null;
}

/**
 * 地図の大きさの倍率（1 が元の大きさ）。地図が上に固定されてから下にスクロールした分だけ縮め、上に戻すと元に戻す。
 * `sentinel` は固定する見出しの直前に置いた要素（それがどれだけ上に過ぎたかで測る）
 */
export function useShrinkingScale(
  sentinel: React.RefObject<HTMLDivElement | null>,
) {
  const [scale, setScale] = useState(1);
  useEffect(() => {
    const container = scrollParent(sentinel.current);
    if (!container) return;
    let frame = 0;
    const update = () => {
      frame = 0;
      const vh = window.innerHeight;
      const large = vh * MAP_HEIGHT.large;
      const small = vh * MAP_HEIGHT.small;
      const past =
        container.getBoundingClientRect().top -
        (sentinel.current?.getBoundingClientRect().top ?? 0);
      const height = Math.min(large, Math.max(small, large - past));
      setScale(Math.round((height / large) * 1000) / 1000);
    };
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update);
    };
    update();
    container.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      cancelAnimationFrame(frame);
      container.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  }, [sentinel]);
  return scale;
}
