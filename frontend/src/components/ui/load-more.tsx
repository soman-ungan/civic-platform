// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect, useRef } from "react";

import { ERROR_TEXT } from "./styles";

/**
 * 一覧の終わりの印。画面に近づいたら続きを読む（無限スクロール）。
 * 読んだあとも印が見えたまま（一覧が短い）なら続けて読むよう、件数が変わるたびに見張り直す
 */
export function LoadMore({
  count,
  done,
  loading,
  error,
  onLoad,
}: {
  /** 今の件数（変わったら見張り直す） */
  count: number;
  done: boolean;
  loading: boolean;
  error: boolean;
  onLoad: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || done || error) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) onLoad();
      },
      { rootMargin: "600px 0px" },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [count, done, error, onLoad]);

  if (done) return null;
  return (
    <div ref={ref} className="py-4 text-center text-sm text-zinc-500">
      {error ? (
        <button
          type="button"
          onClick={onLoad}
          className={`${ERROR_TEXT} underline underline-offset-2`}
        >
          読み込めませんでした。もう一度読む
        </button>
      ) : loading ? (
        "読み込み中…"
      ) : null}
    </div>
  );
}
