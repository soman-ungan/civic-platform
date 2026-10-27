// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { FinanceBars } from "@/components/finance-bars";
import { FinanceNature } from "@/components/finance-nature";
import { BudgetComparison } from "@/components/budget-comparison";
import { BudgetProjects } from "@/components/budget-projects";
import {
  type CarouselSection,
  FinanceCarousel,
} from "@/components/finance-carousel";
import { FinanceTrend } from "@/components/finance-trend";
import { InfoTip } from "@/components/info-tip";
import { PageSlide } from "@/components/page-slide";
import { YearChips } from "@/components/year-chips";
import { apiGet } from "@/lib/api-server";
import { termNote } from "@/lib/finance-terms";
import {
  SOURCE_NOTE,
  fiscalYearLabel,
  roughYen,
  thousandYen,
} from "@/lib/money";
import type {
  FinanceIndicator,
  Finances,
  MunicipalityDetail,
} from "@/lib/types";

export default function FinancesPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/finances">) {
  // 自治体名とタブは (tabs)/layout.tsx の固定のヘッダー。ここでは年度と1人あたりの数字も固定し、
  // その下だけをスクロールする
  return (
    <PageSlide>
      <main className="flex min-h-0 flex-1 flex-col">
        <Suspense fallback={<FinancesSkeleton />}>
          <FinancesView params={params} searchParams={searchParams} />
        </Suspense>
      </main>
    </PageSlide>
  );
}

async function FinancesView({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/finances">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { year } = await searchParams;
  const path = `/api/municipalities/${encodeURIComponent(slug)}`;
  const query =
    typeof year === "string" ? `?year=${encodeURIComponent(year)}` : "";
  const [municipality, f] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<Finances>(`${path}/finances${query}`),
  ]);
  if (!municipality || !f) {
    notFound();
  }
  const s = f.settlement;
  const b = f.budget;
  // 決算があれば決算、無ければ予算（今年度など）の数字で見せる
  const kind = s ? "決算" : "予算";
  const total = s ? s.expenditure_total : (b?.total ?? null);
  const population = s ? s.population : (b?.population ?? null);
  const perCapita =
    total !== null && population ? (total * 1000) / population : null;
  const purpose = s?.expenditure_purpose ?? b?.expenditure_purpose ?? [];
  const nature = s?.expenditure_nature ?? b?.expenditure_nature ?? [];
  const revenue = s?.revenue ?? b?.revenue ?? [];
  const revenueTotal = s ? s.revenue_total : (b?.total ?? null);
  const lastTrend = f.trend.at(-1);
  const latestOrder = Object.entries(lastTrend?.expenditure_purpose ?? {})
    .filter(([, v]) => (v ?? 0) > 0)
    .sort(([, a], [, b]) => (b ?? 0) - (a ?? 0))
    .map(([k]) => k);

  const sections: CarouselSection[] = [];
  if (f.trend.length >= 2) {
    sections.push({
      id: "trend",
      title: "これまでの推移",
      lead: (
        <>
          決算の、分野ごとの歳出を積み上げています。費目を選ぶと、表示・非表示を切り替えられます。
          {f.trend.some((t) => t.fiscal_year === 2020) &&
            "令和2年度は、国の特別定額給付金（1人10万円）などで、どの自治体も大きく増えています。"}
        </>
      ),
      content: (
        <FinanceTrend
          trend={f.trend}
          latestOrder={latestOrder}
          selectedYear={f.fiscal_year}
        />
      ),
    });
  }
  if (b && b.projects.length > 0) {
    sections.push({
      id: "projects",
      title: "今年の主な事業",
      lead: (
        <>
          {fiscalYearLabel(f.fiscal_year)}
          の予算の主な事業（金額の大きい順）。事業の説明は、原本のページをご覧ください。
        </>
      ),
      content: <BudgetProjects projects={b.projects} pdfUrl={b.source.url} />,
    });
  }
  sections.push(
    {
      id: "purpose",
      title: "何に使ったか",
      lead: `分野ごとの歳出（目的別・${kind}）。1人あたりの額と、全体に占める割合です。`,
      content: <FinanceBars kind="purpose" items={purpose} donut />,
    },
    {
      id: "nature",
      title: "お金の使い方で見る",
      lead: `同じ歳出を、使い方で分けたもの（性質別・${kind}）。行を開くと中身が出ます。`,
      content: <FinanceNature items={nature} />,
    },
    {
      id: "revenue",
      title: "どこから来たか",
      lead: (
        <>
          歳入（{kind}。総額{" "}
          {revenueTotal !== null ? thousandYen(revenueTotal) : "—"}
          ）。税のほか、国や県からのお金、借り入れ（地方債）などです。
        </>
      ),
      content: <FinanceBars kind="revenue" items={revenue} limit={8} donut />,
    },
  );
  if (s && b) {
    sections.push({
      id: "compare",
      title: "予算と決算",
      lead: `${fiscalYearLabel(f.fiscal_year)}に、当初予算で見込んだ額と、決算で実際に使った額です（目的別）。`,
      content: (
        <BudgetComparison
          budget={b.expenditure_purpose}
          settlement={s.expenditure_purpose}
        />
      ),
    });
  }
  if (s && s.indicators.length > 0) {
    sections.push({
      id: "indicators",
      title: "財政の指標",
      content: (
        <dl className="flex flex-col gap-2">
          {s.indicators.map((i) => {
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
      ),
    });
  }

  return (
    <>
      <div className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 py-2">
          <YearChips
            slug={municipality.slug}
            years={f.years}
            selected={f.fiscal_year}
          />
          <p className="mt-2 flex flex-wrap items-baseline gap-x-2">
            <span className="text-xs text-zinc-500">
              {fiscalYearLabel(f.fiscal_year)}・
              <span
                className={
                  s
                    ? undefined
                    : "rounded bg-amber-100 px-1 text-amber-900 dark:bg-amber-900/60 dark:text-amber-100"
                }
              >
                {kind}
              </span>
              ・1人あたり
            </span>
            <span className="text-2xl font-bold tabular-nums tracking-tight">
              {perCapita !== null ? roughYen(perCapita) : "—"}
            </span>
            <span className="text-xs text-zinc-500">
              総額 {total !== null ? thousandYen(total) : "—"}・人口{" "}
              {population?.toLocaleString("ja-JP") ?? "—"}人
              {!s && b?.population_year && `（${b.population_year}年度）`}
            </span>
          </p>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-md px-4 pt-2 pb-6">
          <p className="mb-1 text-xs leading-5 text-zinc-500">
            {municipality.name}
            のお金の使い方です（{kind}
            ）。左右にフリックすると、ほかの見方に切り替わります。ⓘ
            で説明が出ます。
          </p>

          <FinanceCarousel sections={sections} />

          <footer className="mt-4 flex flex-col gap-2 text-xs leading-5 text-zinc-500">
            {s && (
              <p>
                {SOURCE_NOTE}（{s.source.title}
                {s.source.page_url && (
                  <>
                    、
                    <a
                      href={s.source.page_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="underline underline-offset-2"
                    >
                      総務省のページ
                    </a>
                  </>
                )}
                ）。1人あたりの額は、総額をその年度の1月1日の住民基本台帳の人口で割ったものです。
              </p>
            )}
            {b && (
              <p>
                予算は、{municipality.name}の「
                {b.source.url ? (
                  <a
                    href={b.source.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="underline underline-offset-2"
                  >
                    {b.source.title}
                  </a>
                ) : (
                  b.source.title
                )}
                」から、数字と事業の名前を取り出して作成しました（AI
                で読み取り、本文と照らし合わせて確かめています）。
                {!s &&
                  b.population_year &&
                  `1人あたりの額は、${b.population_year}年度の人口で割ったものです（予算の年度の人口はまだ公表されていないため）。`}
              </p>
            )}
          </footer>
        </div>
      </div>
    </>
  );
}

function indicatorText(i: FinanceIndicator): string {
  if (i.value === null) return "—";
  if (i.unit === "thousand_yen") return thousandYen(i.value);
  return i.key === "財政力指数"
    ? i.value.toFixed(2)
    : `${i.value.toLocaleString("ja-JP", { maximumFractionDigits: 1 })}%`;
}

function FinancesSkeleton() {
  return (
    <div
      aria-hidden
      className="mx-auto w-full max-w-md animate-pulse px-4 py-2"
    >
      <div className="mb-3 h-9 rounded-full bg-zinc-100 dark:bg-zinc-900" />
      <div className="mb-3 h-7 w-32 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-8 h-28 rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
      <div className="h-64 rounded bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
