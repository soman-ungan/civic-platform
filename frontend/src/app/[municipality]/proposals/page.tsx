// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { ProposalPanel } from "@/components/proposals/panel";
import { municipalityApi, proposalsApi } from "@/lib/api/paths";
import { apiGet, requestCookie } from "@/lib/api/server";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import type { Proposals } from "@/lib/api/types/proposals";

export const metadata: Metadata = {
  title: "トピックを提案する",
  robots: { index: false },
};

/** 市民からのトピックの提案（フェーズ6d。ADR 0005）。提案のフォームと、自分の提案の一覧 */
export default function ProposalsPage({
  params,
}: PageProps<"/[municipality]/proposals">) {
  // 画面の高さは固定（app/layout.tsx）なので、戻るリンクと見出しを固定のヘッダーにし、その下をスクロールする
  return (
    <main className="flex min-h-0 flex-1 flex-col">
      <Suspense>
        <ProposalsView params={params} />
      </Suspense>
    </main>
  );
}

async function ProposalsView({
  params,
}: Pick<PageProps<"/[municipality]/proposals">, "params">) {
  const { municipality: slug } = await params;
  // 提案できるかと自分の提案は参加者ごとに違うので、Cookie を backend に渡す
  const cookie = await requestCookie();
  const [municipality, proposals] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<Proposals>(proposalsApi(slug), { cookie }),
  ]);
  if (!municipality || !proposals) {
    notFound();
  }

  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pb-3">
          <Link
            href={`/${municipality.slug}`}
            className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
          >
            ← {municipality.name}のトピック
          </Link>
          <h1 className="text-lg leading-snug font-bold">トピックを提案する</h1>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-md px-4 pt-3 pb-10">
          <p className="mb-5 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
            {municipality.name}
            について、みんなで考えたい課題はありませんか。議会で話し合われていない課題も提案できます。
          </p>
          <ProposalPanel slug={municipality.slug} initial={proposals} />
        </div>
      </div>
    </>
  );
}
