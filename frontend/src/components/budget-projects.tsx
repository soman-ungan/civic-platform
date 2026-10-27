// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { roughYen, thousandYen } from "@/lib/money";
import type { BudgetProject } from "@/lib/types";

/** 最初に出す事業の数。残りは「さらに表示」で出す */
const INITIAL = 20;
const NEW_OR_EXPANDED = "__flagged";

/**
 * 当初予算の主な事業（金額の大きい順）。分野タグ・新規/拡充で絞り込める。
 * 事業の説明の文章は載せない（市の著作物のため）。原本の PDF のそのページへリンクする
 */
export function BudgetProjects({
  projects,
  pdfUrl,
}: {
  projects: BudgetProject[];
  pdfUrl: string | null;
}) {
  const [filter, setFilter] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);

  const tags = new Map<string, string>();
  for (const p of projects) for (const t of p.tags) tags.set(t.id, t.name);
  const flagged = projects.filter((p) => p.flag !== null).length;
  const visible = projects.filter((p) =>
    filter === null
      ? true
      : filter === NEW_OR_EXPANDED
        ? p.flag !== null
        : p.tags.some((t) => t.id === filter),
  );
  const shown = expanded ? visible : visible.slice(0, INITIAL);

  const chips: [string | null, string][] = [
    [null, `すべて（${projects.length}）`],
    ...(flagged > 0
      ? ([[NEW_OR_EXPANDED, `新規・拡充（${flagged}）`]] as [string, string][])
      : []),
    ...[...tags.entries()].map(([id, name]) => [id, name] as [string, string]),
  ];

  return (
    <div>
      <nav
        aria-label="分野で絞り込む"
        data-swipe-ignore
        className="-mx-4 mb-3 overflow-x-auto px-4 [scrollbar-width:none]"
      >
        <ul className="flex w-max gap-1.5">
          {chips.map(([id, label]) => (
            <li key={id ?? "all"}>
              <button
                type="button"
                aria-pressed={filter === id}
                onClick={() => {
                  setFilter(id);
                  setExpanded(false);
                }}
                className={`inline-flex min-h-9 items-center rounded-full border px-3 text-xs whitespace-nowrap ${
                  filter === id
                    ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
                    : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
                }`}
              >
                {id !== null && id !== NEW_OR_EXPANDED && "#"}
                {label}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      <ul className="flex flex-col divide-y divide-zinc-100 dark:divide-zinc-800">
        {shown.map((p) => (
          <li key={`${p.name}-${p.page}`} className="py-2.5">
            <div className="flex items-baseline justify-between gap-3">
              <p className="text-sm leading-6">
                {p.flag && (
                  <span className="mr-1.5 inline-block rounded bg-amber-100 px-1.5 text-xs font-medium text-amber-900 dark:bg-amber-900/60 dark:text-amber-100">
                    {p.flag === "new" ? "新規" : "拡充"}
                  </span>
                )}
                {p.name}
              </p>
              <p className="shrink-0 text-right text-sm tabular-nums">
                {thousandYen(p.amount)}
                {p.per_capita !== null && (
                  <span className="block text-xs text-zinc-500">
                    1人 {roughYen(p.per_capita)}
                  </span>
                )}
              </p>
            </div>
            <p className="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-zinc-500">
              {p.tags.map((t) => (
                <span key={t.id} className="text-sky-700 dark:text-sky-400">
                  #{t.name}
                </span>
              ))}
              {p.plan_label && <span>{p.plan_label}</span>}
              {pdfUrl && (
                <a
                  href={`${pdfUrl}#page=${p.page}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="ml-auto underline underline-offset-2"
                >
                  説明は原本 p.{p.page}
                </a>
              )}
            </p>
          </li>
        ))}
      </ul>
      {visible.length > shown.length && (
        <button
          type="button"
          onClick={() => setExpanded(true)}
          className="mt-2 h-10 w-full rounded-full border border-zinc-300 text-sm text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
        >
          さらに表示（残り{visible.length - shown.length}件）
        </button>
      )}
    </div>
  );
}
