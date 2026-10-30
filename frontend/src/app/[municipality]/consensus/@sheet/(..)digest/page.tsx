// SPDX-License-Identifier: AGPL-3.0-only

import { Suspense } from "react";

import { DigestDetailView, DigestSkeleton } from "@/components/digest/detail";
import { BottomSheet } from "@/components/layout/bottom-sheet";

/**
 * 合意点のページから開いたまとめ（ページの上に重ねるレイヤ。最初から全体の高さ）。
 * URL は普通のまとめのページと同じ /{slug}/digest?anchor=&topics=。共有・再読み込みでは普通のページが開く
 */
export default function DigestSheet({
  params,
  searchParams,
}: PageProps<"/[municipality]/digest">) {
  // params・searchParams を読むのは Suspense の中（外で読むと、レイヤの枠ごと表示が止まる）
  return (
    <BottomSheet label="まとめ" initialSize="full">
      <Suspense fallback={<DigestSkeleton />}>
        <SheetContent params={params} searchParams={searchParams} />
      </Suspense>
    </BottomSheet>
  );
}

async function SheetContent({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/digest">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { anchor, topics } = await searchParams;
  return (
    <DigestDetailView slug={slug} anchor={anchor} topics={topics} inSheet />
  );
}
