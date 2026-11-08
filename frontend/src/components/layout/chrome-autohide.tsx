// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { usePathname } from "next/navigation";
import { useEffect } from "react";

/** この分だけ同じ向きにスクロールしたら切り替える（少し動いただけで出たり隠れたりしないように） */
const THRESHOLD_PX = 32;
/** 切り替えたあと、たたむ動き（globals.css の .chrome）が終わるまでは次の切り替えをしない */
const LOCK_MS = 300;
/** 一番上とみなす近さ */
const EDGE_PX = 4;

/**
 * 下にスクロールしたらヘッダーとフッター（`.chrome` で包んだもの）をたたみ、上にスクロールしたら出す。
 * 一番上に来たときも出す。一番下では出さない（2026-11-08。読んでいる途中で画面が動かないように）。
 * フッターのソースコードのリンク（AGPL 第13条）は、上にスクロールすればいつでも出る。
 * スクロールの枠はページごとに違うので、document で scroll を capture して、どの枠の縦のスクロールも拾う。
 * 状態は `<html data-chrome="hidden">` だけ（ページを移ったら出す）。何も描かない
 */
export function ChromeAutoHide() {
  const pathname = usePathname();

  useEffect(() => {
    const root = document.documentElement;
    const last = new WeakMap<Element, number>();
    let moved = 0;
    let lockedUntil = 0;

    const set = (hidden: boolean) => {
      if ((root.dataset.chrome === "hidden") === hidden) return;
      if (hidden) root.dataset.chrome = "hidden";
      else delete root.dataset.chrome;
      lockedUntil = performance.now() + LOCK_MS;
      moved = 0;
    };

    const onScroll = (e: Event) => {
      const el = e.target === document ? document.scrollingElement : e.target;
      // 入力欄の中のスクロールは数えない
      if (!(el instanceof Element) || el instanceof HTMLTextAreaElement) {
        return;
      }
      const top = el.scrollTop;
      const prev = last.get(el);
      last.set(el, top);
      // 横のスクロール（カルーセルなど）は縦の位置が変わらない
      if (prev === undefined || top === prev) return;
      // たたむ・広げる動きで枠の高さが変わり、位置が動いた分は数えない
      if (performance.now() < lockedUntil) return;

      if (top <= EDGE_PX) {
        set(false);
        return;
      }
      const delta = top - prev;
      // 向きが変わったら数え直す
      moved = Math.sign(delta) === Math.sign(moved) ? moved + delta : delta;
      if (moved > THRESHOLD_PX) set(true);
      else if (moved < -THRESHOLD_PX) set(false);
    };

    document.addEventListener("scroll", onScroll, {
      capture: true,
      passive: true,
    });
    return () =>
      document.removeEventListener("scroll", onScroll, { capture: true });
  }, []);

  // ページを移ったら出す（新しいページの一番上にいるので）
  useEffect(() => {
    delete document.documentElement.dataset.chrome;
  }, [pathname]);

  return null;
}
