// SPDX-License-Identifier: AGPL-3.0-only

import { Suspense } from "react";

import { BottomSheet } from "@/components/digest/sheet";
import { TopicDetailView, TopicSkeleton } from "@/components/topics/detail";

/**
 * 合意点のページからトピックを開いたときに、ページの上に重ねるレイヤ（インターセプトルート）。
 * URL は普通のトピックの画面と同じ /{slug}/topics/{id}。共有・再読み込みでは普通の画面が開く
 */
export default async function TopicSheet({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  const { municipality: slug, id } = await params;
  return (
    <BottomSheet label="トピックの詳細">
      <Suspense fallback={<TopicSkeleton />}>
        <TopicDetailView slug={slug} id={id} />
      </Suspense>
    </BottomSheet>
  );
}
