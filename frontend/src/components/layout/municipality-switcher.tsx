// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";

import type { Municipality } from "@/lib/api/types/municipalities";

/**
 * ヘッダーの自治体名。押すと、公開中の自治体の一覧（都道府県ごと）を開き、選ぶとその自治体の同じタブへ移る。
 * 一覧は backend の公開中の自治体（`GET /api/municipalities`）で、自治体を書き込まない。
 * 1つだけの間も同じ形で出す（フェーズ8で自治体を足したときに、そのまま使えるように）
 */
export function MunicipalitySwitcher({
  current,
  municipalities,
}: {
  current: Municipality;
  municipalities: Municipality[];
}) {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();
  const root = useRef<HTMLDivElement>(null);

  // 一覧の外を押す・Esc で閉じる
  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  /** 同じタブのまま自治体だけを替えた URL（/{slug}、/{slug}/finances など） */
  function hrefFor(slug: string): string {
    const rest = pathname.split("/").slice(2).join("/");
    return rest ? `/${slug}/${rest}` : `/${slug}`;
  }

  const byPrefecture = new Map<string, Municipality[]>();
  for (const m of municipalities) {
    byPrefecture.set(m.prefecture_name, [
      ...(byPrefecture.get(m.prefecture_name) ?? []),
      m,
    ]);
  }

  return (
    <div ref={root} className="relative">
      <p className="text-xs text-zinc-500">{current.prefecture_name}</p>
      <h1 className="text-xl font-bold tracking-tight">
        <button
          type="button"
          aria-expanded={open}
          aria-haspopup="true"
          onClick={() => setOpen((o) => !o)}
          className="flex items-center gap-1"
        >
          {current.name}
          <span aria-hidden className="text-sm font-normal text-zinc-500">
            {open ? "▴" : "▾"}
          </span>
          <span className="sr-only">（自治体を切り替える）</span>
        </button>
      </h1>
      {open && (
        <div className="absolute top-full left-0 z-40 mt-2 max-h-[60dvh] w-64 overflow-y-auto rounded-2xl border border-zinc-200 bg-[var(--background)] p-2 shadow-lg dark:border-zinc-800">
          <p className="px-2 pt-1 pb-2 text-xs text-zinc-500">自治体を選ぶ</p>
          {[...byPrefecture].map(([prefecture, items]) => (
            <section key={prefecture} className="mb-1">
              <h2 className="px-2 py-1 text-xs font-semibold text-zinc-500">
                {prefecture}
              </h2>
              <ul>
                {items.map((m) => {
                  const here = m.slug === current.slug;
                  return (
                    <li key={m.id}>
                      <Link
                        href={hrefFor(m.slug)}
                        aria-current={here ? "page" : undefined}
                        onClick={() => setOpen(false)}
                        className={`flex h-11 items-center justify-between rounded-xl px-3 text-sm ${
                          here
                            ? "bg-zinc-100 font-semibold dark:bg-zinc-800"
                            : "active:bg-zinc-100 dark:active:bg-zinc-800"
                        }`}
                      >
                        {m.name}
                        {here && <span aria-hidden>✓</span>}
                      </Link>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
          {municipalities.length <= 1 && (
            <p className="px-2 pt-1 pb-1 text-xs leading-5 text-zinc-500">
              ほかの自治体は準備中です。
            </p>
          )}
        </div>
      )}
    </div>
  );
}
