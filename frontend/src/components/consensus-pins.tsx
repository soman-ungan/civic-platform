// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import type { MunicipalityConsensus } from "@/lib/types";

/** 最初に表示する件数。残りは「もっと見る」で開く */
const INITIAL_COUNT = 3;

/** タイムライン最上位の「合意点」。全グループの8割以上が賛成している意見 */
export function ConsensusPins({
  slug,
  items,
}: {
  slug: string;
  items: MunicipalityConsensus[];
}) {
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? items : items.slice(0, INITIAL_COUNT);
  const hidden = items.length - INITIAL_COUNT;

  return (
    <section
      aria-labelledby="pins-title"
      className="mb-5 rounded-2xl bg-emerald-50 p-4 dark:bg-emerald-950/40"
    >
      <h2
        id="pins-title"
        className="text-sm font-bold text-emerald-900 dark:text-emerald-200"
      >
        <span aria-hidden>📌 </span>
        みんなの合意点
      </h2>
      <p className="mb-3 text-xs text-emerald-800 dark:text-emerald-300">
        意見の違うグループのどれでも、8割以上が賛成している意見です。
      </p>
      <ul id="pins-list" className="flex flex-col gap-3">
        {visible.map((c) => (
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
      {hidden > 0 && (
        <button
          type="button"
          aria-expanded={expanded}
          aria-controls="pins-list"
          onClick={() => setExpanded((e) => !e)}
          className="mt-3 h-10 w-full rounded-full border border-emerald-300 text-sm text-emerald-900 dark:border-emerald-800 dark:text-emerald-200"
        >
          {expanded ? "閉じる" : `もっと見る（残り${hidden}件）`}
        </button>
      )}
    </section>
  );
}
