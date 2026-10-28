// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useRef, useState } from "react";

import { BudgetTreemap } from "./budget-treemap";
import { ChipButton } from "@/components/ui/chip";
import { ScrollRow } from "@/components/ui/scroll-row";
import { BADGE_AMBER, BUTTON_SECONDARY } from "@/components/ui/styles";
import {
  FLAG_LABEL,
  OTHER_FIELDS,
  fieldGroups,
  otherFieldIds,
} from "@/lib/finances/budget-fields";
import {
  type Unit,
  amountText,
  roughYen,
  thousandYen,
} from "@/lib/finances/money";
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
  unit,
}: {
  projects: BudgetProject[];
  pdfUrl: string | null;
  /** 予算の全体（千円） */
  total: number;
  population: number | null;
  /** 1人あたりか総額か（ヘッダーの切り替え）。もう一方は小さく添える */
  unit: Unit;
}) {
  const [filter, setFilter] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);
  const listTop = useRef<HTMLHeadingElement>(null);

  const groups = fieldGroups(projects);
  const colorOf = new Map(groups.map((g) => [g.id, g.color]));
  const otherIds = otherFieldIds(groups);
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
                  <span
                    className={`self-start rounded px-1.5 text-xs font-medium ${BADGE_AMBER}`}
                  >
                    {p.flag && FLAG_LABEL[p.flag]}
                  </span>
                  <p className="mt-1.5 line-clamp-3 text-sm leading-5 font-medium">
                    {p.name}
                  </p>
                  <p className="mt-auto pt-2 text-sm tabular-nums">
                    {amountText(unit, p.amount, p.per_capita)}
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

      <h3
        ref={listTop}
        className="mt-5 mb-2 scroll-mt-[var(--carousel-header,0px)] text-sm font-semibold"
      >
        事業の一覧
      </h3>
      {/* 一覧を読み進めても分野を選び直せるように、欄の見出しの下に固定する */}
      <ScrollRow
        label="分野で絞り込む"
        className="sticky top-[var(--carousel-header,0px)] z-[5] mb-3 bg-[var(--background)] py-1.5"
      >
        {chips.map(([id, label]) => (
          <li key={id ?? "all"}>
            <ChipButton
              active={filter === id}
              onClick={() => {
                choose(id);
                // 固定されたチップで選び直したとき、短くなった一覧の先頭に戻る（見出しがチップのすぐ上にあれば動かさない）
                const el = listTop.current;
                const nav = el?.nextElementSibling;
                if (
                  el &&
                  nav &&
                  el.getBoundingClientRect().top <
                    nav.getBoundingClientRect().top - el.offsetHeight - 16
                ) {
                  el.scrollIntoView({ block: "start" });
                }
              }}
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
            </ChipButton>
          </li>
        ))}
      </ScrollRow>

      <ul className="flex flex-col divide-y divide-zinc-100 dark:divide-zinc-800">
        {shown.map((p) => (
          <li key={`${p.name}-${p.page}`} className="py-2.5">
            <div className="flex items-baseline justify-between gap-3">
              <p className="text-sm leading-6">
                {p.flag && (
                  <span
                    className={`mr-1.5 inline-block rounded px-1.5 text-xs font-medium ${BADGE_AMBER}`}
                  >
                    {FLAG_LABEL[p.flag]}
                  </span>
                )}
                {p.name}
              </p>
              <p className="shrink-0 text-right text-sm tabular-nums">
                {amountText(unit, p.amount, p.per_capita)}
                {p.per_capita !== null && (
                  <span className="block text-xs text-zinc-500">
                    {unit === "total"
                      ? `1人 ${roughYen(p.per_capita)}`
                      : thousandYen(p.amount)}
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
          className={`mt-2 h-10 w-full text-sm text-zinc-700 dark:text-zinc-300 ${BUTTON_SECONDARY}`}
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
