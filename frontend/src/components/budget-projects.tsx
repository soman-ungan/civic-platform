// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import {
  BudgetTreemap,
  OTHER_FIELDS,
  fieldGroups,
} from "@/components/budget-treemap";
import { roughYen, thousandYen } from "@/lib/money";
import type { BudgetProject } from "@/lib/types";

/** 最初に出す事業の数。残りは「さらに表示」で出す */
const INITIAL = 20;
const NEW_OR_EXPANDED = "__flagged";

/**
 * 当初予算の主な事業。冒頭に新規・拡充のカード、ツリーマップ（分野 → 事業）、その下に一覧（金額の大きい順）。
 * ツリーマップで分野を開くと、一覧もその分野に絞り込む（逆も）。分野をまたぐ事業は、一覧ではどちらの分野にも出す。
 * 事業の説明の文章は載せない（市の著作物のため）。原本の PDF のそのページへリンクする
 */
export function BudgetProjects({
  projects,
  pdfUrl,
  total,
  population,
}: {
  projects: BudgetProject[];
  pdfUrl: string | null;
  /** 予算の全体（千円） */
  total: number;
  population: number | null;
}) {
  const [filter, setFilter] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);

  const groups = fieldGroups(projects);
  const colorOf = new Map(groups.map((g) => [g.id, g.color]));
  const otherIds = new Set(groups.slice(7).map((g) => g.id));
  const flaggedProjects = projects.filter((p) => p.flag !== null);
  const flagged = flaggedProjects.length;
  const visible = projects.filter((p) =>
    filter === null
      ? true
      : filter === NEW_OR_EXPANDED
        ? p.flag !== null
        : filter === OTHER_FIELDS
          ? p.tags.some((t) => otherIds.has(t.id))
          : p.tags.some((t) => t.id === filter),
  );
  const treemapField = filter === NEW_OR_EXPANDED ? null : filter;
  const choose = (id: string | null) => {
    setFilter(id);
    setExpanded(false);
  };
  const shown = expanded ? visible : visible.slice(0, INITIAL);

  const chips: [string | null, string][] = [
    [null, `すべて（${projects.length}）`],
    ...(flagged > 0
      ? ([[NEW_OR_EXPANDED, `新規・拡充（${flagged}）`]] as [string, string][])
      : []),
    ...groups.map((g) => [g.id, g.name] as [string, string]),
  ];

  return (
    <div>
      {flagged > 0 && (
        <section aria-labelledby="new-projects" className="mb-4">
          <h3 id="new-projects" className="mb-2 text-sm font-semibold">
            今年の新しい取り組み
          </h3>
          <ul
            data-swipe-ignore
            className="-mx-4 flex snap-x gap-2 overflow-x-auto px-4 pb-1 [scrollbar-width:none]"
          >
            {flaggedProjects
              .toSorted((a, b) =>
                a.flag === b.flag
                  ? b.amount - a.amount
                  : a.flag === "new"
                    ? -1
                    : 1,
              )
              .map((p) => (
                <li
                  key={`${p.name}-${p.page}`}
                  className="flex w-52 shrink-0 snap-start flex-col rounded-2xl border border-zinc-200 p-3 dark:border-zinc-800"
                >
                  <span className="self-start rounded bg-amber-100 px-1.5 text-xs font-medium text-amber-900 dark:bg-amber-900/60 dark:text-amber-100">
                    {p.flag === "new" ? "新規" : "拡充"}
                  </span>
                  <p className="mt-1.5 line-clamp-3 text-sm leading-5 font-medium">
                    {p.name}
                  </p>
                  <p className="mt-auto pt-2 text-sm tabular-nums">
                    {thousandYen(p.amount)}
                  </p>
                  <p className="mt-0.5 flex items-center gap-2 text-xs text-zinc-500">
                    {p.tags[0] && (
                      <span className="text-sky-700 dark:text-sky-400">
                        #{p.tags[0].name}
                      </span>
                    )}
                    {pdfUrl && (
                      <a
                        href={`${pdfUrl}#page=${p.page}`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="ml-auto underline underline-offset-2"
                      >
                        原本 p.{p.page}
                      </a>
                    )}
                  </p>
                </li>
              ))}
          </ul>
        </section>
      )}

      <BudgetTreemap
        projects={projects}
        groups={groups}
        total={total}
        population={population}
        pdfUrl={pdfUrl}
        field={treemapField}
        onField={choose}
      />

      <h3 className="mt-5 mb-2 text-sm font-semibold">事業の一覧</h3>
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
                onClick={() => choose(id)}
                className={`inline-flex min-h-9 items-center rounded-full border px-3 text-xs whitespace-nowrap ${
                  filter === id
                    ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
                    : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
                }`}
              >
                {colorOf.has(id ?? "") && (
                  <span
                    aria-hidden
                    className="mr-1.5 inline-block h-2.5 w-2.5 rounded-sm"
                    style={{ background: colorOf.get(id ?? "") }}
                  />
                )}
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
      <p className="mt-3 text-xs leading-5 text-zinc-500">
        分野をまたぐ事業は、一覧ではどちらの分野にも出ます。ツリーマップの面積は、最初の分野で数えています。
      </p>
    </div>
  );
}
