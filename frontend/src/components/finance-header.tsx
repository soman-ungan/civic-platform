// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { YearChips } from "@/components/year-chips";
import { type FinanceView, KIND_LABEL, financesHref } from "@/lib/finance-view";
import { fiscalYearLabel, roughYen, thousandYen } from "@/lib/money";
import type { FinanceKind, Finances } from "@/lib/types";

/** 予算の印（決算がまだ無い年度や、予算を選んでいるとき） */
const BUDGET_MARK =
  "rounded bg-amber-100 px-1 text-amber-900 dark:bg-amber-900/60 dark:text-amber-100";

/**
 * まちのお金の固定のヘッダー: 年度のチップ、「決算｜予算」の切り替え、1人あたりと総額。
 * 切り替えは、その年度に決算と予算の両方があるときだけ押せる（片方だけなら、ある方を文字で出す）
 */
export function FinanceHeader({
  slug,
  finances: f,
  view: v,
}: {
  slug: string;
  finances: Finances;
  view: FinanceView;
}) {
  return (
    <div className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
      <div className="mx-auto w-full max-w-md px-4 py-2">
        <YearChips
          slug={slug}
          years={f.years}
          selected={f.fiscal_year}
          view={v.kind}
        />
        <div className="mt-2 flex flex-wrap items-baseline gap-x-2 gap-y-1">
          <span className="flex items-center gap-1.5 text-xs text-zinc-500">
            {fiscalYearLabel(f.fiscal_year)}
            {v.kinds.length > 1 ? (
              <KindSwitch slug={slug} year={f.fiscal_year} current={v.kind} />
            ) : (
              <span className={v.kind === "budget" ? BUDGET_MARK : undefined}>
                {KIND_LABEL[v.kind]}
              </span>
            )}
            1人あたり
          </span>
          <span className="text-2xl font-bold tabular-nums tracking-tight">
            {v.perCapita !== null ? roughYen(v.perCapita) : "—"}
          </span>
          <span className="text-xs text-zinc-500">
            総額 {v.total !== null ? thousandYen(v.total) : "—"}・人口{" "}
            {v.population?.toLocaleString("ja-JP") ?? "—"}人
            {v.kind === "budget" &&
              v.populationYear &&
              `（${v.populationYear}年度）`}
          </span>
        </div>
      </div>
    </div>
  );
}

/** 「決算｜予算」の2つ並びの切り替え（URL の ?view= を変える） */
function KindSwitch({
  slug,
  year,
  current,
}: {
  slug: string;
  year: number;
  current: FinanceKind;
}) {
  return (
    <span
      role="group"
      aria-label="決算と予算の切り替え"
      className="inline-flex rounded-full border border-zinc-300 p-0.5 dark:border-zinc-700"
    >
      {(["settlement", "budget"] as const).map((kind) => (
        <Link
          key={kind}
          href={financesHref(slug, year, kind)}
          scroll={false}
          replace
          aria-current={kind === current ? "true" : undefined}
          className={`inline-flex min-h-8 items-center rounded-full px-3 ${
            kind === current
              ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900"
              : "text-zinc-600 dark:text-zinc-400"
          }`}
        >
          {KIND_LABEL[kind]}
        </Link>
      ))}
    </span>
  );
}
