// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { type RefObject, useCallback, useEffect, useId } from "react";

/** ほかの吹き出しを閉じる合図（同時に開くのは1つだけ） */
const OPEN_EVENT = "infotip-open";

/**
 * 吹き出しの閉じ方をそろえる: `ref` の外に触れたら閉じ、ほかの吹き出しが開いたら閉じる。
 * 返す関数は、開くときに呼ぶ（ほかの吹き出しに閉じる合図を送る）。`close` は毎回同じ関数を渡す
 */
export function useDismiss(
  open: boolean,
  close: () => void,
  ref: RefObject<HTMLElement | null>,
): () => void {
  const id = useId();

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: Event) => {
      // 吹き出しの部品そのものに触れたときは、部品の onClick に任せる（閉じたあとに開き直さないため）
      if (ref.current?.contains(e.target as Node)) return;
      close();
    };
    const onOtherOpen = (e: Event) => {
      if ((e as CustomEvent<string>).detail !== id) close();
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener(OPEN_EVENT, onOtherOpen);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener(OPEN_EVENT, onOtherOpen);
    };
  }, [open, close, ref, id]);

  return useCallback(() => {
    window.dispatchEvent(new CustomEvent(OPEN_EVENT, { detail: id }));
  }, [id]);
}
