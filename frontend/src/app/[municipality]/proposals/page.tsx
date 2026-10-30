// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import {
  ProposalPanel,
  type ProposalSource,
} from "@/components/proposals/panel";
import { digestApi, municipalityApi, proposalsApi } from "@/lib/api/paths";
import { apiGet, requestCookie } from "@/lib/api/server";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import type { Digest } from "@/lib/api/types/digest";
import type { Proposals } from "@/lib/api/types/proposals";
import { idList } from "@/lib/digest";

export const metadata: Metadata = {
  title: "トピックを提案する",
  robots: { index: false },
};

/** 市民からのトピックの提案（フェーズ6d。ADR 0005）。提案のフォームと、自分の提案の一覧 */
export default function ProposalsPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/proposals">) {
  // 画面の高さは固定（app/layout.tsx）なので、戻るリンクと見出しを固定のヘッダーにし、その下をスクロールする
  return (
    <main className="flex min-h-0 flex-1 flex-col">
      <Suspense>
        <ProposalsView params={params} searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function ProposalsView({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/proposals">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { anchor, topics, use } = await searchParams;
  // 提案できるかと自分の提案は参加者ごとに違うので、Cookie を backend に渡す
  const cookie = await requestCookie();
  const [municipality, proposals] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<Proposals>(proposalsApi(slug), { cookie }),
  ]);
  if (!municipality || !proposals) {
    notFound();
  }
  // 市民のまとめから来たとき（?anchor=&topics=&use=）: 元のトピックの題と、使う意見の原文をまとめから読む
  let from: ProposalSource | undefined;
  if (typeof anchor === "string" && anchor !== "") {
    const digest = await apiGet<Digest>(
      digestApi(slug, anchor, idList(topics)),
    ).catch(() => null);
    if (digest) {
      const used = new Set(idList(use));
      const bodies = new Map<string, string>();
      for (const t of digest.topics) {
        for (const item of [...t.consensus, ...t.divided]) {
          bodies.set(item.statement_id, item.body);
        }
        for (const g of t.groups) {
          for (const r of g.representative) bodies.set(r.statement_id, r.body);
        }
      }
      from = {
        parents: digest.topics.map((t) => ({ id: t.id, title: t.title })),
        statements: [...used].flatMap((id) => bodies.get(id) ?? []),
      };
    }
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
          <ProposalPanel
            slug={municipality.slug}
            initial={proposals}
            from={from}
          />
        </div>
      </div>
    </>
  );
}
