// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect, useId, useRef, useState } from "react";

/** ほかの吹き出しを閉じる合図（同時に開くのは1つだけ） */
const OPEN_EVENT = "infotip-open";

/**
 * ⓘ のボタンと、短い説明の吹き出し。タップで開き、画面のどこかに触れると閉じる
 * （もう一度 ⓘ に触れても閉じる）。触れる範囲は見た目より広い 44px
 */
export function InfoTip({ label, note }: { label: string; note: string }) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const ref = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: Event) => {
      // ⓘ そのものに触れたときは、ボタンの onClick に任せる（閉じたあとに開き直さないため）
      if (ref.current?.contains(e.target as Node)) return;
      setOpen(false);
    };
    const others = (e: Event) => {
      if ((e as CustomEvent<string>).detail !== id) setOpen(false);
    };
    document.addEventListener("pointerdown", close, true);
    window.addEventListener(OPEN_EVENT, others);
    return () => {
      document.removeEventListener("pointerdown", close, true);
      window.removeEventListener(OPEN_EVENT, others);
    };
  }, [open, id]);

  function toggle(e: React.MouseEvent) {
    // 開閉できる欄の見出しの中に置いても、欄が開閉しないように
    e.preventDefault();
    e.stopPropagation();
    setOpen((o) => {
      if (!o) window.dispatchEvent(new CustomEvent(OPEN_EVENT, { detail: id }));
      return !o;
    });
  }

  return (
    <span ref={ref} className="relative inline-flex align-middle">
      <button
        type="button"
        aria-label={`${label}の説明`}
        aria-expanded={open}
        aria-describedby={open ? id : undefined}
        onClick={toggle}
        className="-m-3 inline-flex h-11 w-11 items-center justify-center text-zinc-400 hover:text-zinc-600 dark:text-zinc-500 dark:hover:text-zinc-300"
      >
        <svg aria-hidden viewBox="0 0 16 16" className="h-4 w-4">
          <circle
            cx="8"
            cy="8"
            r="7"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          />
          <circle cx="8" cy="4.8" r="1" fill="currentColor" />
          <path
            d="M8 7.2v4.6"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
          />
        </svg>
      </button>
      {open && (
        <span
          id={id}
          role="tooltip"
          className="absolute top-full left-0 z-30 mt-2 w-64 max-w-[calc(100vw-3rem)] rounded-xl bg-zinc-900 px-3 py-2 text-xs leading-5 font-normal text-white shadow-lg dark:bg-zinc-100 dark:text-zinc-900"
        >
          {note}
        </span>
      )}
    </span>
  );
}
