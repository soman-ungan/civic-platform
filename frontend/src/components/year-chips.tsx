// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect, useRef } from "react";

import { ChipLink } from "@/components/ui/chip";
import { ScrollRow } from "@/components/ui/scroll-row";
import { financesHref } from "@/lib/finance-view";
import type { Unit } from "@/lib/money";
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
  unit,
}: {
  slug: string;
  years: FinanceYear[];
  selected: number;
  view: FinanceKind;
  unit: Unit;
}) {
  const current = useRef<HTMLAnchorElement>(null);
  useEffect(() => {
    current.current?.scrollIntoView({ inline: "center", block: "nearest" });
  }, [selected]);

  return (
    <ScrollRow label="年度">
      {[...years].reverse().map(({ fiscal_year: y, settlement }) => {
        const active = y === selected;
        return (
          <li key={y}>
            <ChipLink
              ref={active ? current : undefined}
              href={financesHref(slug, y, view, unit)}
              active={active}
              className="tabular-nums"
            >
              {y}年度
              {/* 決算がまだ無く、予算だけの年度 */}
              {!settlement && <span className="ml-1 opacity-70">予算</span>}
            </ChipLink>
          </li>
        );
      })}
    </ScrollRow>
  );
}
