// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { PageSlide } from "@/components/layout/page-slide";
import { RecommendationList } from "@/components/recommendations/list";
import { recommendationsApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { RecommendationSummary } from "@/lib/api/types/recommendations";

export default function RecommendationsPage({
  params,
}: PageProps<"/[municipality]/recommendations">) {
  // 自治体名とタブは (tabs)/layout.tsx の固定のヘッダー。ここでは提言のカードの一覧だけをスクロールさせる
  return (
    <PageSlide>
      <main className="mx-auto flex min-h-0 w-full max-w-md flex-1 flex-col px-4 pt-3">
        <Suspense fallback={<ListSkeleton />}>
          <Recommendations params={params} />
        </Suspense>
      </main>
    </PageSlide>
  );
}

async function Recommendations({
  params,
}: Pick<PageProps<"/[municipality]/recommendations">, "params">) {
  const { municipality: slug } = await params;
  const items = await apiGet<RecommendationSummary[]>(recommendationsApi(slug));
  if (!items) {
    notFound();
  }
  return <RecommendationList slug={slug} items={items} />;
}

function ListSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-3 h-10 rounded bg-zinc-100 dark:bg-zinc-900" />
      {[0, 1].map((i) => (
        <div
          key={i}
          className="mb-3 h-24 rounded-2xl bg-zinc-100 dark:bg-zinc-900"
        />
      ))}
    </div>
  );
}
