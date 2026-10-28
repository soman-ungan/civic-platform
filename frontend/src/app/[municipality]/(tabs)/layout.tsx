// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { SwipeNav } from "@/components/layout/swipe-nav";
import { TabNav, TabNavFallback } from "@/components/layout/tab-nav";
import { municipalityApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";

/**
 * 自治体のトップ（トピック・まちのお金）の共通の枠。自治体名とタブを固定のヘッダーとして常に表示し、
 * 本体は左右のスワイプで切り替える。URL はタブごとに別（共有された URL だけで開ける）
 */
export default function TabsLayout({
  children,
  params,
}: LayoutProps<"/[municipality]">) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pt-3">
          <Suspense fallback={<NameSkeleton />}>
            <MunicipalityName params={params} />
          </Suspense>
          <Suspense fallback={<TabNavFallback />}>
            <TabNav />
          </Suspense>
        </div>
      </header>
      <SwipeNav>{children}</SwipeNav>
    </div>
  );
}

async function MunicipalityName({
  params,
}: Pick<LayoutProps<"/[municipality]">, "params">) {
  const { municipality: slug } = await params;
  const municipality = await apiGet<MunicipalityDetail>(municipalityApi(slug));
  if (!municipality) {
    notFound();
  }
  return (
    <div>
      <p className="text-xs text-zinc-500">{municipality.prefecture_name}</p>
      <h1 className="text-xl font-bold tracking-tight">{municipality.name}</h1>
    </div>
  );
}

function NameSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-1 h-3 w-12 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="h-7 w-28 rounded bg-zinc-200 dark:bg-zinc-800" />
    </div>
  );
}
