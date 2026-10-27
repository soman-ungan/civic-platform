// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { FinanceBars } from "@/components/finance-bars";
import { FinanceTrend } from "@/components/finance-trend";
import { PageSlide } from "@/components/page-slide";
import { apiGet } from "@/lib/api-server";
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

/** 財政指標の説明（中立に、意味だけを書く） */
const INDICATOR_NOTES: Record<string, string> = {
  財政力指数:
    "必要な費用を、自前の税収などでどれだけまかなえるか。1に近いほど自前でまかなえる",
  経常収支比率:
    "毎年決まって入るお金のうち、人件費や借金の返済など毎年決まって出ていく費用に使う割合。高いほど自由に使えるお金が少ない",
  実質公債費比率:
    "収入に対する借金の返済の重さ。18%以上になると、借り入れに国や県の許可が要る",
  将来負担比率:
    "これから払う借金などの重さ。市町村は350%以上で、財政を立て直す計画が必要になる",
  実質収支比率: "その年度の黒字（赤字）の大きさ",
  地方債現在高: "年度末の借金の残り",
};

export default function FinancesPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/finances">) {
  // 自治体名とタブは (tabs)/layout.tsx の固定のヘッダー。この下は全体をスクロールする
  return (
    <PageSlide>
      <main className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-md px-4 py-4">
          <Suspense fallback={<FinancesSkeleton />}>
            <FinancesView params={params} searchParams={searchParams} />
          </Suspense>
        </div>
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
  const purposeKeys = [...f.expenditure_purpose]
    .filter((i) => (i.value ?? 0) > 0)
    .sort((a, b) => (b.value ?? 0) - (a.value ?? 0))
    .slice(0, 6)
    .map((i) => i.key);

  return (
    <>
      <p className="mb-3 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
        {municipality.name}
        が1年間に使ったお金（決算）を、1人あたりにして見ています。
      </p>

      <nav aria-label="年度" className="mb-5 flex flex-wrap gap-1.5">
        {[...f.years].reverse().map((y) => (
          <Link
            key={y}
            href={`/${municipality.slug}/finances?year=${y}`}
            aria-current={y === f.fiscal_year ? "page" : undefined}
            className={`inline-flex min-h-9 items-center rounded-full border px-3 text-xs tabular-nums ${
              y === f.fiscal_year
                ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
                : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
            }`}
          >
            {y}
          </Link>
        ))}
      </nav>

      <section className="mb-8 rounded-2xl bg-zinc-50 p-4 dark:bg-zinc-900">
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          {fiscalYearLabel(f.fiscal_year)}（{f.fiscal_year}年度）に使ったお金は
        </p>
        <p className="mt-1">
          <span className="text-sm">1人あたり </span>
          <span className="text-4xl font-bold tabular-nums tracking-tight">
            {perCapita !== null ? roughYen(perCapita) : "—"}
          </span>
        </p>
        <p className="mt-2 text-xs leading-5 text-zinc-500">
          総額{" "}
          {f.expenditure_total !== null
            ? thousandYen(f.expenditure_total)
            : "—"}
          ・人口 {f.population?.toLocaleString("ja-JP") ?? "—"}
          人（住民基本台帳）
        </p>
      </section>

      <section aria-labelledby="purpose" className="mb-8">
        <h2 id="purpose" className="mb-1 font-semibold">
          何に使ったか
        </h2>
        <p className="mb-3 text-xs leading-5 text-zinc-500">
          目的別の歳出。1人あたりの額と、全体に占める割合です。
        </p>
        <FinanceBars items={f.expenditure_purpose} />
      </section>

      <section aria-labelledby="trend" className="mb-8">
        <h2 id="trend" className="mb-1 font-semibold">
          これまでの推移
        </h2>
        <p className="mb-3 text-xs leading-5 text-zinc-500">
          1人あたりの歳出。
          {f.years.includes(2020) &&
            "令和2年度は、国の特別定額給付金（1人10万円）などで、どの自治体も大きく増えています。"}
        </p>
        <FinanceTrend trend={f.trend} keys={purposeKeys} />
      </section>

      <section aria-labelledby="revenue" className="mb-8">
        <h2 id="revenue" className="mb-1 font-semibold">
          どこから来たか
        </h2>
        <p className="mb-3 text-xs leading-5 text-zinc-500">
          歳入（総額{" "}
          {f.revenue_total !== null ? thousandYen(f.revenue_total) : "—"}
          ）。市税のほか、国や県からのお金、借り入れ（地方債）などです。
        </p>
        <FinanceBars items={f.revenue} limit={8} />
      </section>

      <details className="mb-8">
        <summary className="flex min-h-11 cursor-pointer items-center font-semibold">
          お金の性質で見る
        </summary>
        <p className="mb-3 text-xs leading-5 text-zinc-500">
          性質別の歳出。人件費・扶助費（福祉の給付など）・公債費（借金の返済）は、毎年決まって出ていく費用です。
        </p>
        <FinanceBars items={f.expenditure_nature} />
      </details>

      {f.indicators.length > 0 && (
        <section aria-labelledby="indicators" className="mb-8">
          <h2 id="indicators" className="mb-3 font-semibold">
            財政の指標
          </h2>
          <dl className="flex flex-col gap-3">
            {f.indicators.map((i) => (
              <div key={i.key}>
                <div className="flex items-baseline justify-between gap-2 text-sm">
                  <dt className="font-medium">{i.key}</dt>
                  <dd className="tabular-nums">{indicatorText(i)}</dd>
                </div>
                {INDICATOR_NOTES[i.key] && (
                  <p className="mt-0.5 text-xs leading-5 text-zinc-500">
                    {INDICATOR_NOTES[i.key]}
                  </p>
                )}
              </div>
            ))}
          </dl>
        </section>
      )}

      <footer className="text-xs leading-5 text-zinc-500">
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
    <div aria-hidden className="animate-pulse">
      <div className="mb-6 h-4 w-40 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-7 w-32 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-8 h-28 rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
      <div className="h-64 rounded bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
