// SPDX-License-Identifier: AGPL-3.0-only

import { SegmentedSwitch } from "@/components/ui/segmented-switch";
import { BADGE_AMBER } from "@/components/ui/styles";
import { YearChips } from "@/components/year-chips";
import { type FinanceView, KIND_LABEL, financesHref } from "@/lib/finance-view";
import {
  UNIT_LABEL,
  type Unit,
  fiscalYearLabel,
  roughYen,
  thousandYen,
} from "@/lib/money";
import type { FinanceKind, Finances } from "@/lib/types";

/** 予算の印（決算がまだ無い年度や、予算を選んでいるとき） */
const BUDGET_MARK = `rounded px-1 ${BADGE_AMBER}`;

/**
 * まちのお金の固定のヘッダー: 年度のチップ、「決算｜予算」と「1人あたり｜総額」の切り替え、大きな数字。
 * 決算と予算の切り替えは、その年度に両方あるときだけ押せる（片方だけなら、ある方を文字で出す）。
 * 1人あたりと総額の切り替えは、すべての欄の金額に効く
 */
export function FinanceHeader({
  slug,
  finances: f,
  view: v,
  unit,
}: {
  slug: string;
  finances: Finances;
  view: FinanceView;
  unit: Unit;
}) {
  const href = (kind: FinanceKind, u: Unit) =>
    financesHref(slug, f.fiscal_year, kind, u);
  const perCapita = v.perCapita !== null ? roughYen(v.perCapita) : "—";
  const total = v.total !== null ? thousandYen(v.total) : "—";
  return (
    <div className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
      <div className="mx-auto w-full max-w-md px-4 py-2">
        <YearChips
          slug={slug}
          years={f.years}
          selected={f.fiscal_year}
          view={v.kind}
          unit={unit}
        />
        <div className="mt-2 flex items-center justify-between gap-2 text-xs text-zinc-500">
          <span className="flex items-center gap-1.5">
            {fiscalYearLabel(f.fiscal_year)}
            {v.kinds.length > 1 ? (
              <SegmentedSwitch
                label="決算と予算の切り替え"
                options={(["settlement", "budget"] as const).map((kind) => ({
                  key: kind,
                  label: KIND_LABEL[kind],
                  href: href(kind, unit),
                }))}
                current={v.kind}
              />
            ) : (
              <span className={v.kind === "budget" ? BUDGET_MARK : undefined}>
                {KIND_LABEL[v.kind]}
              </span>
            )}
          </span>
          <SegmentedSwitch
            label="金額の見せ方"
            options={(["per_capita", "total"] as const).map((u) => ({
              key: u,
              label: UNIT_LABEL[u],
              href: href(v.kind, u),
            }))}
            current={unit}
          />
        </div>
        <p className="mt-1 flex flex-wrap items-baseline gap-x-2">
          <span className="text-xs text-zinc-500">{UNIT_LABEL[unit]}</span>
          <span className="text-2xl font-bold tabular-nums tracking-tight">
            {unit === "total" ? total : perCapita}
          </span>
          <span className="text-xs text-zinc-500">
            {unit === "total" ? `1人あたり ${perCapita}` : `総額 ${total}`}
            ・人口 {v.population?.toLocaleString("ja-JP") ?? "—"}人
            {v.kind === "budget" &&
              v.populationYear &&
              `（${v.populationYear}年度）`}
          </span>
        </p>
      </div>
    </div>
  );
}
