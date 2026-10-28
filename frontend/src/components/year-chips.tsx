// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useEffect, useRef } from "react";

import { financesHref } from "@/lib/finance-view";
import type { FinanceKind, FinanceYear } from "@/lib/types";

/**
 * 年度のボタン（1行。横にスクロールして選ぶ）。選んでいる年度が見えるように、表示したときに寄せる。
 * 横のスクロールをタブのスワイプと取り違えないように `data-swipe-ignore`。
 * 「決算／予算」の切り替えは、年度を変えても引き継ぐ（その年度に無ければ、ある方が出る）
 */
export function YearChips({
  slug,
  years,
  selected,
  view,
}: {
  slug: string;
  years: FinanceYear[];
  selected: number;
  view: FinanceKind;
}) {
  const current = useRef<HTMLAnchorElement>(null);
  useEffect(() => {
    current.current?.scrollIntoView({ inline: "center", block: "nearest" });
  }, [selected]);

  return (
    <nav
      aria-label="年度"
      data-swipe-ignore
      className="-mx-4 overflow-x-auto px-4 [scrollbar-width:none]"
    >
      <ul className="flex w-max gap-1.5">
        {[...years].reverse().map(({ fiscal_year: y, settlement }) => {
          const active = y === selected;
          return (
            <li key={y}>
              <Link
                ref={active ? current : undefined}
                href={financesHref(slug, y, view)}
                scroll={false}
                aria-current={active ? "page" : undefined}
                className={`inline-flex min-h-9 items-center rounded-full border px-3 text-xs tabular-nums ${
                  active
                    ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
                    : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
                }`}
              >
                {y}年度
                {/* 決算がまだ無く、予算だけの年度 */}
                {!settlement && <span className="ml-1 opacity-70">予算</span>}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
