// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { PageSlide } from "@/components/layout/page-slide";
import { OfficialListView } from "@/components/officials/list";
import { municipalityApi, officialsApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import type { OfficialList } from "@/lib/api/types/officials";
import { municipalityMetadata } from "@/lib/metadata";

export function generateMetadata({
  params,
}: PageProps<"/[municipality]/council">) {
  return municipalityMetadata(
    params,
    (name) => `${name}の議会`,
    (name) =>
      `${name}の議員と首長が議会で取り上げたことと、そこから生まれたトピックを、会議録の引用で見られます。`,
  );
}

/** タブ「議会」: 議員の記録の一覧（ADR 0008）。自治体名とタブは (tabs)/layout.tsx の固定のヘッダー */
export default function CouncilPage({
  params,
}: PageProps<"/[municipality]/council">) {
  return (
    <PageSlide>
      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-md px-4 pt-3 pb-6">
          <Suspense fallback={<CouncilSkeleton />}>
            <Council params={params} />
          </Suspense>
        </div>
      </main>
    </PageSlide>
  );
}

async function Council({
  params,
}: Pick<PageProps<"/[municipality]/council">, "params">) {
  const { municipality: slug } = await params;
  const [m, list] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<OfficialList>(officialsApi(slug)),
  ]);
  if (!m || !list) {
    notFound();
  }
  return <OfficialListView slug={slug} municipalityName={m.name} list={list} />;
}

function CouncilSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-3 h-10 rounded bg-zinc-100 dark:bg-zinc-900" />
      <div className="h-96 rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
