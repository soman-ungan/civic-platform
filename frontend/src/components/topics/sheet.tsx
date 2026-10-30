// SPDX-License-Identifier: AGPL-3.0-only

import { Suspense } from "react";

import { BottomSheet } from "@/components/layout/bottom-sheet";
import { TopicDetailView, TopicSkeleton } from "@/components/topics/detail";

/**
 * トピック詳細を、開いたページの上に下から伸びるレイヤで重ねる（インターセプトルートのページの中身）。
 * タイムライン（`(tabs)/@sheet/(.)topics/[id]`）と合意点のページ（`consensus/@sheet/(..)topics/[id]`）で使う。
 * URL は普通のトピックの画面と同じ /{slug}/topics/{id}。共有・再読み込みでは普通の画面が開く
 */
export function TopicSheet({
  params,
}: Pick<PageProps<"/[municipality]/topics/[id]">, "params">) {
  // params を読むのは Suspense の中（外で読むと、レイヤの枠ごと表示が止まる）
  return (
    <BottomSheet label="トピックの詳細">
      <Suspense fallback={<TopicSkeleton />}>
        <SheetContent params={params} />
      </Suspense>
    </BottomSheet>
  );
}

async function SheetContent({
  params,
}: Pick<PageProps<"/[municipality]/topics/[id]">, "params">) {
  const { municipality: slug, id } = await params;
  return <TopicDetailView slug={slug} id={id} />;
}
