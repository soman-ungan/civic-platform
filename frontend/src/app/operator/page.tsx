// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { OperatorConsole } from "@/components/operator/console";
import { OPERATOR_MUNICIPALITIES_API } from "@/lib/api/paths";
import { apiGet, requestCookie } from "@/lib/api/server";
import type { Municipality } from "@/lib/api/types/municipalities";
import { OPERATOR_TABS, type OperatorTab } from "@/lib/operator";

export const metadata: Metadata = {
  title: "管理画面",
  robots: { index: false },
};

/**
 * 運営者の管理画面（フェーズ6c）: 下書きのトピック・確認待ちの意見・公開中のトピック。
 * 自治体（?m=）とタブ（?tab=）は URL に持つ。運営者でなければ 404（backend が 404 を返す）
 */
export default function OperatorPage({ searchParams }: PageProps<"/operator">) {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto">
      <Suspense>
        <Console searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function Console({
  searchParams,
}: Pick<PageProps<"/operator">, "searchParams">) {
  const municipalities = await apiGet<Municipality[]>(
    OPERATOR_MUNICIPALITIES_API,
    { cookie: await requestCookie() },
  );
  if (!municipalities || municipalities.length === 0) {
    notFound();
  }
  const { m, tab } = await searchParams;
  const municipality =
    municipalities.find((x) => x.slug === m) ?? municipalities[0];
  const current: OperatorTab =
    OPERATOR_TABS.find((t) => t.key === tab)?.key ?? "drafts";
  return (
    <OperatorConsole
      municipalities={municipalities}
      municipality={municipality}
      tab={current}
    />
  );
}
