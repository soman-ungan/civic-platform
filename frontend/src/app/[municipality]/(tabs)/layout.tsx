// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { OperatorLink } from "@/components/auth/operator-link";
import { SwipeNav } from "@/components/layout/swipe-nav";
import { TabNav, TabNavFallback } from "@/components/layout/tab-nav";
import { MunicipalitySwitcher } from "@/components/layout/municipality-switcher";
import { MUNICIPALITIES_API, municipalityApi } from "@/lib/api/paths";
import { MUNICIPALITY_NAME_ID } from "@/lib/tabs";
import { apiGet } from "@/lib/api/server";
import type {
  Municipality,
  MunicipalityDetail,
} from "@/lib/api/types/municipalities";

/**
 * 自治体のトップ（トピック・まちのお金）の共通の枠。自治体名とタブを固定のヘッダーとして常に表示し、
 * 本体は左右のスワイプで切り替える。URL はタブごとに別（共有された URL だけで開ける）。
 * `sheet` は、ここから開いたトピック詳細を上に重ねるレイヤ（`@sheet/(.)topics/[id]`。開いていなければ何も出さない）
 */
export default function TabsLayout({
  children,
  sheet,
  params,
}: LayoutProps<"/[municipality]">) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pt-3">
          <div
            id={MUNICIPALITY_NAME_ID}
            className="flex items-start justify-between gap-4"
          >
            <Suspense fallback={<NameSkeleton />}>
              <MunicipalityName params={params} />
            </Suspense>
            <OperatorLink />
          </div>
          <Suspense fallback={<TabNavFallback />}>
            <TabNav />
          </Suspense>
        </div>
      </header>
      <SwipeNav>{children}</SwipeNav>
      {sheet}
    </div>
  );
}

async function MunicipalityName({
  params,
}: Pick<LayoutProps<"/[municipality]">, "params">) {
  const { municipality: slug } = await params;
  const [municipality, all] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<Municipality[]>(MUNICIPALITIES_API),
  ]);
  if (!municipality) {
    notFound();
  }
  // 自治体名を押すと、公開中の自治体の一覧から切り替えられる
  return (
    <MunicipalitySwitcher
      current={municipality}
      municipalities={all ?? [municipality]}
    />
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
