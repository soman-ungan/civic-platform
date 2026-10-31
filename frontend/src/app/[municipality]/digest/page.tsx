// SPDX-License-Identifier: AGPL-3.0-only

import { Suspense } from "react";

import { DigestDetailView, DigestSkeleton } from "@/components/digest/detail";
import { municipalityMetadata } from "@/lib/metadata";

/** 市民のまとめ（?anchor=起点&topics=近傍から選んだトピック）。保存しないので、URL で共有・印刷できる */
export function generateMetadata({
  params,
}: PageProps<"/[municipality]/digest">) {
  return municipalityMetadata(
    params,
    (name) => `${name}の議論のまとめ`,
    () =>
      "締め切ったトピックの合意点・意見が分かれた点・グループの代表的な意見を、原文と数字のまま見られます。",
  );
}

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
