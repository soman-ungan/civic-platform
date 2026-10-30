// SPDX-License-Identifier: AGPL-3.0-only

import { Suspense } from "react";

import { DigestDetailView, DigestSkeleton } from "@/components/digest/detail";

/** 市民のまとめ（?anchor=起点&topics=近傍から選んだトピック）。保存しないので、URL で共有・印刷できる */
export default function DigestPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/digest">) {
  // 見出しは固定のヘッダーにし、中身だけをスクロールする。印刷のときは全体を1枚の文書として流す
  return (
    <main className="flex min-h-0 flex-1 flex-col print:block">
      <Suspense fallback={<DigestSkeleton />}>
        <DigestPageView params={params} searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function DigestPageView({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/digest">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { anchor, topics } = await searchParams;
  return <DigestDetailView slug={slug} anchor={anchor} topics={topics} />;
}
