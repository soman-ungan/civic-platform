// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import type { MunicipalityConsensus } from "@/lib/api/types/analysis";
import { consensusMapHref } from "@/lib/digest";

/**
 * 合意点（全グループの8割以上が賛成している意見）。見出しの1行だけを出し、押すと開く（中身はページの中に伸ばす）。
 * 元のトピックが締め切ったものなら、押すと下の地図でそのトピックを起点にして示す。受付中ならトピックへ
 */
export function ConsensusToggle({
  slug,
  items,
}: {
  slug: string;
  items: MunicipalityConsensus[];
}) {
  const [open, setOpen] = useState(false);
  return (
    <section
      aria-labelledby="consensus-title"
      className="rounded-2xl bg-emerald-50 dark:bg-emerald-950/40"
    >
      <h2 id="consensus-title">
        <button
          type="button"
          aria-expanded={open}
          aria-controls="consensus-body"
          onClick={() => setOpen((o) => !o)}
          className="flex min-h-11 w-full items-center justify-between gap-2 px-4 text-left text-sm font-bold text-emerald-900 dark:text-emerald-200"
        >
          <span>
            <span aria-hidden>📌 </span>
            合意点（{items.length}件）
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
        <div id="consensus-body" className="px-4 pb-4">
          <p className="mb-3 text-xs text-emerald-800 dark:text-emerald-300">
            意見の違うグループのどれでも、8割以上が賛成している意見です。締め切ったトピックの合意点は、押すと下の地図でそのトピックを示します。
          </p>
          {items.length === 0 ? (
            <p className="text-sm text-emerald-900 dark:text-emerald-200">
              まだ合意点はありません。トピックの最初の意見すべてに投票すると、そのトピックの合意点がここに出ます。
            </p>
          ) : (
            <ul className="flex flex-col gap-3">
              {items.map((c) => (
                <li key={c.statement_id}>
                  <Link
                    href={
                      c.closed
                        ? consensusMapHref(slug, { focus: c.topic_id })
                        : `/${slug}/topics/${c.topic_id}`
                    }
                    scroll={!c.closed}
                    onClick={() => setOpen(false)}
                    className="block"
                  >
                    <p className="leading-7">{c.body}</p>
                    <p className="flex items-baseline justify-between gap-2 text-xs text-emerald-800 dark:text-emerald-300">
                      <span className="min-w-0 underline underline-offset-2">
                        {c.topic_title}
                      </span>
                      <span className="shrink-0">
                        {c.closed ? "地図で見る ↓" : "トピックへ →"}
                      </span>
                    </p>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
