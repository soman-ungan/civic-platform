// SPDX-License-Identifier: AGPL-3.0-only

import { BudgetComparison } from "./budget-comparison";
import { BudgetProjects } from "./budget-projects";
import type { CarouselSection } from "./carousel";
import { FinanceBars } from "./bars";
import { FinanceNature } from "./nature";
import { FinanceTrend } from "./trend";
import { InfoTip } from "@/components/ui/info-tip";
import { EmptyState } from "@/components/ui/empty-state";
import { termNote } from "@/lib/finances/terms";
import { type FinanceView, KIND_LABEL } from "@/lib/finances/view";
import { type Unit, fiscalYearLabel, thousandYen } from "@/lib/finances/money";
import type { FinanceIndicator, Finances } from "@/lib/types";

/** 決算がまだ無い年度の説明 */
const NO_SETTLEMENT =
  "この年度の決算は、まだ公表されていません（決算は、年度が終わった翌年の秋ごろに公表されます）。";
const NO_BUDGET = "この年度の予算の概要は、まだ取り込んでいません。";

/**
 * まちのお金の欄（カルーセル）。年度によらず、いつも同じ欄を同じ順で並べ、
 * その年度にデータが無い欄は、消さずに理由を出す（欄の数や順番が年度で変わらないように）。
 * 「何に使ったか」「お金の使い方で見る」「どこから来たか」は、ヘッダーの「決算／予算」の切り替えに従う
 */
export function financeSections(
  f: Finances,
  v: FinanceView,
  unit: Unit,
): CarouselSection[] {
  const s = f.settlement;
  const b = f.budget;
  const kind = KIND_LABEL[v.kind];
  const year = fiscalYearLabel(f.fiscal_year);
  const lastTrend = f.trend.at(-1);
  const latestOrder = Object.entries(lastTrend?.expenditure_purpose ?? {})
    .filter(([, value]) => (value ?? 0) > 0)
    .sort(([, x], [, y]) => (y ?? 0) - (x ?? 0))
    .map(([key]) => key);
  const lacking = (items: unknown[]) =>
    items.length === 0 ? (
      <EmptyState>この年度の{kind}には、この内訳がありません。</EmptyState>
    ) : null;

  return [
    {
      id: "trend",
      title: "これまでの推移",
      lead: (
        <>
          分野ごとの歳出を積み上げています（決算。決算がまだ無い年度は当初予算）。費目を選ぶと、表示・非表示を切り替えられます。
          {f.trend.some((t) => t.fiscal_year === 2020) &&
            "令和2年度は、国の特別定額給付金（1人10万円）などで、どの自治体も大きく増えています。"}
        </>
      ),
      content:
        f.trend.length >= 2 ? (
          <FinanceTrend
            trend={f.trend}
            latestOrder={latestOrder}
            selectedYear={f.fiscal_year}
            view={v.kind}
            unit={unit}
          />
        ) : (
          <EmptyState>推移は、2つ以上の年度の数字がそろうと出ます。</EmptyState>
        ),
    },
    {
      id: "purpose",
      title: "何に使ったか",
      lead: `分野ごとの歳出（目的別・${kind}）。1人あたりの額と、全体に占める割合です。`,
      content: lacking(v.purpose) ?? (
        <FinanceBars kind="purpose" items={v.purpose} unit={unit} donut />
      ),
    },
    {
      id: "nature",
      title: "お金の使い方で見る",
      lead: `同じ歳出を、使い方で分けたもの（性質別・${kind}）。行を開くと中身が出ます。`,
      content: lacking(v.nature) ?? (
        <FinanceNature items={v.nature} unit={unit} />
      ),
    },
    {
      id: "revenue",
      title: "どこから来たか",
      lead: (
        <>
          歳入（{kind}。総額{" "}
          {v.revenueTotal !== null ? thousandYen(v.revenueTotal) : "—"}
          ）。税のほか、国や県からのお金、借り入れ（地方債）などです。
        </>
      ),
      content: lacking(v.revenue) ?? (
        <FinanceBars
          kind="revenue"
          items={v.revenue}
          unit={unit}
          limit={8}
          donut
        />
      ),
    },
    {
      id: "projects",
      title: "主な事業（予算）",
      lead: `${year}の当初予算の主な事業。事業の説明は、原本のページをご覧ください。`,
      content: !b ? (
        <EmptyState>{NO_BUDGET}</EmptyState>
      ) : b.projects.length === 0 ? (
        <EmptyState>
          この年度の予算の概要からは、主な事業を取り出せていません。
        </EmptyState>
      ) : (
        <BudgetProjects
          projects={b.projects}
          pdfUrl={b.source.url}
          total={b.total}
          population={b.population}
          unit={unit}
        />
      ),
    },
    {
      id: "compare",
      title: "予算と決算",
      lead: `${year}に、当初予算で見込んだ額と、決算で実際に使った額です（目的別）。`,
      content: !s ? (
        <EmptyState>{NO_SETTLEMENT}</EmptyState>
      ) : !b ? (
        <EmptyState>{NO_BUDGET}</EmptyState>
      ) : (
        <BudgetComparison
          budget={b.expenditure_purpose}
          settlement={s.expenditure_purpose}
          unit={unit}
        />
      ),
    },
    {
      id: "indicators",
      title: "財政の指標",
      lead: "決算から計算される、財政の健全さの目安です。",
      content: !s ? (
        <EmptyState>{NO_SETTLEMENT}</EmptyState>
      ) : s.indicators.length === 0 ? (
        <EmptyState>この年度の決算には、指標がありません。</EmptyState>
      ) : (
        <Indicators indicators={s.indicators} />
      ),
    },
  ];
}

function Indicators({ indicators }: { indicators: FinanceIndicator[] }) {
  return (
    <dl className="flex flex-col gap-2">
      {indicators.map((i) => {
        const note = termNote("indicator", i.key);
        return (
          <div
            key={i.key}
            className="flex items-center justify-between gap-2 text-sm"
          >
            <dt className="flex items-center gap-1 font-medium">
              {i.key}
              {note && <InfoTip label={i.key} note={note} />}
            </dt>
            <dd className="tabular-nums">{indicatorText(i)}</dd>
          </div>
        );
      })}
    </dl>
  );
}

function indicatorText(i: FinanceIndicator): string {
  if (i.value === null) return "—";
  if (i.unit === "thousand_yen") return thousandYen(i.value);
  return i.key === "財政力指数"
    ? i.value.toFixed(2)
    : `${i.value.toLocaleString("ja-JP", { maximumFractionDigits: 1 })}%`;
}
