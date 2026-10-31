// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { FinanceCarousel } from "@/components/finances/carousel";
import { FinanceHeader } from "@/components/finances/header";
import { financeSections } from "@/components/finances/sections";
import { FinanceSources } from "@/components/finances/sources";
import { PageSlide } from "@/components/layout/page-slide";
import { EmptyState } from "@/components/ui/empty-state";
import { municipalityApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import { KIND_LABEL, financeView, parseUnit } from "@/lib/finances/view";
import type { Finances } from "@/lib/api/types/finances";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import { municipalityMetadata } from "@/lib/metadata";

export function generateMetadata({
  params,
}: PageProps<"/[municipality]/finances">) {
  return municipalityMetadata(
    params,
    (name) => `${name}のお金`,
    (name) =>
      `${name}の決算と当初予算を、分野ごと・1人あたりの金額で見られます。`,
  );
}

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
  const { year, view: requested, unit: unitParam } = await searchParams;
  const path = municipalityApi(slug);
  const query =
    typeof year === "string" ? `?year=${encodeURIComponent(year)}` : "";
  const [municipality, f] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<Finances>(`${path}/finances${query}`),
  ]);
  if (!municipality) {
    notFound();
  }
  if (!f) {
    // 決算も予算もまだ取り込んでいない自治体（自治体を切り替えたときなど）
    return (
      <div className="mx-auto w-full max-w-md px-4 pt-3">
        <EmptyState>
          {municipality.name}のお金のデータは、まだ取り込んでいません。
        </EmptyState>
      </div>
    );
  }
  // 決算か予算か（URL の ?view=。無ければ決算、その年度に無ければある方）
  const v = financeView(
    f,
    typeof requested === "string" ? requested : undefined,
  );
  const unit = parseUnit(unitParam);

  return (
    <>
      <FinanceHeader
        slug={municipality.slug}
        finances={f}
        view={v}
        unit={unit}
      />

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-md px-4 pt-2 pb-6">
          <p className="mb-1 text-xs leading-5 text-zinc-500">
            {municipality.name}
            のお金の使い方です（{KIND_LABEL[v.kind]}
            ）。左右にフリックすると、ほかの見方に切り替わります。ⓘ
            で説明が出ます。
          </p>

          <FinanceCarousel sections={financeSections(f, v, unit)} />

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
