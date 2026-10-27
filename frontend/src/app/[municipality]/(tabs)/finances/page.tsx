// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { FinanceBars } from "@/components/finance-bars";
import { FinanceNature } from "@/components/finance-nature";
import { FinanceSection } from "@/components/finance-section";
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
  const perCapita =
    f.expenditure_total !== null && f.population
      ? (f.expenditure_total * 1000) / f.population
      : null;
  const latestOrder = [...f.expenditure_purpose]
    .filter((i) => (i.value ?? 0) > 0)
    .sort((a, b) => (b.value ?? 0) - (a.value ?? 0))
    .map((i) => i.key);

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
              {fiscalYearLabel(f.fiscal_year)}・1人あたり
            </span>
            <span className="text-2xl font-bold tabular-nums tracking-tight">
              {perCapita !== null ? roughYen(perCapita) : "—"}
            </span>
            <span className="text-xs text-zinc-500">
              総額{" "}
              {f.expenditure_total !== null
                ? thousandYen(f.expenditure_total)
                : "—"}
              ・人口 {f.population?.toLocaleString("ja-JP") ?? "—"}人
            </span>
          </p>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-md px-4 pt-2 pb-6">
          <p className="mb-1 text-xs leading-5 text-zinc-500">
            {municipality.name}
            が1年間に使ったお金（決算）を、1人あたりにして見ています。ⓘ
            で説明が出ます。
          </p>

          <FinanceSection
            title="これまでの推移"
            defaultOpen
            lead={
              <>
                分野ごとの歳出を積み上げています。費目を選ぶと、表示・非表示を切り替えられます。
                {f.years.includes(2020) &&
                  "令和2年度は、国の特別定額給付金（1人10万円）などで、どの自治体も大きく増えています。"}
              </>
            }
          >
            <FinanceTrend
              trend={f.trend}
              latestOrder={latestOrder}
              selectedYear={f.fiscal_year}
            />
          </FinanceSection>

          <FinanceSection
            title="何に使ったか"
            lead="分野ごとの歳出（目的別）。1人あたりの額と、全体に占める割合です。"
          >
            <FinanceBars kind="purpose" items={f.expenditure_purpose} donut />
          </FinanceSection>

          <FinanceSection
            title="お金の使い方で見る"
            lead="同じ歳出を、使い方で分けたもの（性質別）。行を開くと中身が出ます。"
          >
            <FinanceNature items={f.expenditure_nature} />
          </FinanceSection>

          <FinanceSection
            title="どこから来たか"
            lead={
              <>
                歳入（総額{" "}
                {f.revenue_total !== null ? thousandYen(f.revenue_total) : "—"}
                ）。税のほか、国や県からのお金、借り入れ（地方債）などです。
              </>
            }
          >
            <FinanceBars kind="revenue" items={f.revenue} limit={8} />
          </FinanceSection>

          {f.indicators.length > 0 && (
            <FinanceSection title="財政の指標">
              <dl className="flex flex-col gap-2">
                {f.indicators.map((i) => {
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
            </FinanceSection>
          )}

          <footer className="mt-4 text-xs leading-5 text-zinc-500">
            <p>
              {SOURCE_NOTE}（{f.source.title}
              {f.source.page_url && (
                <>
                  、
                  <a
                    href={f.source.page_url}
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
