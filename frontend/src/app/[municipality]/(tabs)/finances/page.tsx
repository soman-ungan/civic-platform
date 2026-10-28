// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { FinanceCarousel } from "@/components/finance-carousel";
import { FinanceHeader } from "@/components/finance-header";
import { financeSections } from "@/components/finance-sections";
import { FinanceSources } from "@/components/finance-sources";
import { PageSlide } from "@/components/page-slide";
import { apiGet } from "@/lib/api-server";
import { KIND_LABEL, financeView } from "@/lib/finance-view";
import type { Finances, MunicipalityDetail } from "@/lib/types";

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
  const { year, view: requested } = await searchParams;
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
  // 決算か予算か（URL の ?view=。無ければ決算、その年度に無ければある方）
  const v = financeView(
    f,
    typeof requested === "string" ? requested : undefined,
  );

  return (
    <>
      <FinanceHeader slug={municipality.slug} finances={f} view={v} />

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-md px-4 pt-2 pb-6">
          <p className="mb-1 text-xs leading-5 text-zinc-500">
            {municipality.name}
            のお金の使い方です（{KIND_LABEL[v.kind]}
            ）。左右にフリックすると、ほかの見方に切り替わります。ⓘ
            で説明が出ます。
          </p>

          <FinanceCarousel sections={financeSections(f, v)} />

          <FinanceSources
            municipalityName={municipality.name}
            finances={f}
            view={v}
          />
        </div>
      </div>
    </>
  );
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
