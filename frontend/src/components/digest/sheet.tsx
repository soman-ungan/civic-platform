// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";

/** レイヤの高さ（画面に対する割合）。半分と全体の2段 */
const HEIGHTS = { half: 0.55, full: 0.92 } as const;
type Size = keyof typeof HEIGHTS;
/** つまみをこれ以上動かしたら、段を変えるか閉じる（px） */
const STEP_PX = 60;

/**
 * 合意点のページの上に重ねる、下から伸びるレイヤ（トピック詳細）。
 * 上端のつまみを上下に引っ張ると、半分と全体の2段を行き来し、半分から下まで引っ張ると閉じる。
 * 閉じるとブラウザの「戻る」と同じく合意点のページに戻る（下のページはそのまま残っている）
 */
export function BottomSheet({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  const router = useRouter();
  const [size, setSize] = useState<Size>("half");
  const [entered, setEntered] = useState(false);
  const [drag, setDrag] = useState(0);
  const [dragging, setDragging] = useState(false);
  const start = useRef<number | null>(null);
  /** 引っ張ったあとの click では段を切り替えない */
  const moved = useRef(false);

  // 開いたときに下から伸ばす
  useEffect(() => {
    const id = requestAnimationFrame(() => setEntered(true));
    return () => cancelAnimationFrame(id);
  }, []);

  function close() {
    router.back();
  }

  function onPointerDown(e: React.PointerEvent) {
    start.current = e.clientY;
    moved.current = false;
    setDragging(true);
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: React.PointerEvent) {
    if (start.current === null) return;
    const d = e.clientY - start.current;
    if (Math.abs(d) > 5) moved.current = true;
    setDrag(d);
  }
  function onPointerUp() {
    if (start.current === null) return;
    start.current = null;
    setDragging(false);
    const d = drag;
    setDrag(0);
    if (d < -STEP_PX) setSize("full");
    else if (d > STEP_PX) {
      if (size === "full") setSize("half");
      else close();
    }
  }

  const height = `${HEIGHTS[size] * 100}dvh`;
  return (
    <div className="fixed inset-0 z-40 print:hidden">
      <button
        type="button"
        aria-label="閉じる"
        onClick={close}
        className={`absolute inset-0 bg-black/30 transition-opacity ${entered ? "opacity-100" : "opacity-0"}`}
      />
      <section
        role="dialog"
        aria-modal="true"
        aria-label={label}
        style={{
          height,
          transform: entered
            ? `translateY(${Math.max(drag, -STEP_PX * 2)}px)`
            : "translateY(100%)",
          transition: dragging ? "none" : "transform 200ms, height 200ms",
        }}
        className="absolute inset-x-0 bottom-0 mx-auto flex max-w-md flex-col rounded-t-3xl bg-white shadow-2xl dark:bg-zinc-950"
      >
        <div className="flex shrink-0 items-center justify-between px-2">
          <span className="w-10" />
          <button
            type="button"
            aria-label={
              size === "half"
                ? "上に引っ張ると広がり、下に引っ張ると閉じます"
                : "下に引っ張ると小さくなります"
            }
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onClick={() => {
              if (!moved.current)
                setSize((s) => (s === "half" ? "full" : "half"));
            }}
            className="flex h-8 flex-1 touch-none items-center justify-center"
          >
            <span className="h-1.5 w-10 rounded-full bg-zinc-300 dark:bg-zinc-700" />
          </button>
          <button
            type="button"
            onClick={close}
            aria-label="閉じて合意点のページに戻る"
            className="flex h-10 w-10 items-center justify-center text-lg text-zinc-500"
          >
            ×
          </button>
        </div>
        <div className="flex min-h-0 flex-1 flex-col">{children}</div>
      </section>
    </div>
  );
}
