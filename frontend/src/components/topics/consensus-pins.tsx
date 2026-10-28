// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import type { MunicipalityConsensus } from "@/lib/types";

/**
 * タイムラインの上の「合意点」。全グループの8割以上が賛成している意見。
 * ヘッダーとして常に表示するので、最初は見出しの1行だけにし、タップで開く。
 * 開いた中身は枠の中でスクロールする（カードの一覧の場所を奪いすぎないように）
 */
export function ConsensusPins({
  slug,
  items,
}: {
  slug: string;
  items: MunicipalityConsensus[];
}) {
  const [open, setOpen] = useState(false);

  return (
    <section
      aria-labelledby="pins-title"
      className="mb-3 shrink-0 rounded-2xl bg-emerald-50 dark:bg-emerald-950/40"
    >
      <h2 id="pins-title">
        <button
          type="button"
          aria-expanded={open}
          aria-controls="pins-body"
          onClick={() => setOpen((o) => !o)}
          className="flex min-h-11 w-full items-center justify-between gap-2 px-4 text-left text-sm font-bold text-emerald-900 dark:text-emerald-200"
        >
          <span>
            <span aria-hidden>📌 </span>
            みんなの合意点（{items.length}件）
          </span>
          <span
            aria-hidden
            className={`text-xs transition-transform ${open ? "rotate-180" : ""}`}
          >
            ▼
          </span>
        </button>
      </h2>
      {open && (
        <div
          id="pins-body"
          className="max-h-[40dvh] overflow-y-auto overscroll-contain px-4 pb-4"
        >
          <p className="mb-3 text-xs text-emerald-800 dark:text-emerald-300">
            意見の違うグループのどれでも、8割以上が賛成している意見です。
          </p>
          <ul className="flex flex-col gap-3">
            {items.map((c) => (
              <li key={c.statement_id}>
                <Link href={`/${slug}/topics/${c.topic_id}`} className="block">
                  <p className="leading-7">{c.body}</p>
                  <p className="text-xs text-emerald-800 underline underline-offset-2 dark:text-emerald-300">
                    {c.topic_title}
                  </p>
                </Link>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
