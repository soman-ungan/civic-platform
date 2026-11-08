// SPDX-License-Identifier: AGPL-3.0-only

import { SegmentedSwitch } from "@/components/ui/segmented-switch";
import { BADGE_AMBER } from "@/components/ui/styles";
import { YearChips } from "./year-chips";
import {
  type FinanceView,
  KIND_LABEL,
  financesHref,
} from "@/lib/finances/view";
import {
  UNIT_LABEL,
  type Unit,
  roughYen,
  thousandYen,
} from "@/lib/finances/money";
import type { FinanceKind, Finances } from "@/lib/api/types/finances";

/** 予算の印（決算がまだ無い年度や、予算を選んでいるとき） */
const BUDGET_MARK = `rounded px-1 ${BADGE_AMBER}`;

/**
 * まちのお金の固定のヘッダー: 年度のチップ、「決算｜予算」の切り替え、「1人あたり｜総額」の切り替えと大きな数字・人口。
 * 縦の幅を取らないよう、年度の文字・金額の見出し・もう片方の金額は出さない（2026-11-08。docs/plans/2026-11-08-ui-chrome.md）。
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
        {/* 1行に: 金額の見せ方の切り替え（見出しを兼ねる）・金額と、その下に小さく人口・右に決算か予算か。
            年度はチップで分かるので、文字では出さない */}
        <div className="mt-2 flex items-center gap-3">
          <span className="shrink-0 text-xs">
            <SegmentedSwitch
              label="金額の見せ方"
              options={(["total", "per_capita"] as const).map((u) => ({
                key: u,
                label: UNIT_LABEL[u],
                href: href(v.kind, u),
              }))}
              current={unit}
            />
          </span>
          <p className="min-w-0 leading-tight">
            <span className="block text-2xl font-bold tabular-nums tracking-tight">
              {unit === "total" ? total : perCapita}
            </span>
            <span className="block text-[11px] text-zinc-500">
              人口 {v.population?.toLocaleString("ja-JP") ?? "—"}人
              {v.kind === "budget" &&
                v.populationYear &&
                `（${v.populationYear}年度）`}
            </span>
          </p>
          <span className="ml-auto shrink-0 text-xs text-zinc-500">
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
        </div>
      </div>
    </div>
  );
}
