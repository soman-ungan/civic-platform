// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { TopicMapView } from "@/components/digest/map";
import { PageSlide } from "@/components/layout/page-slide";
import { topicMapApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { TopicMap } from "@/lib/api/types/digest";

export default function MapPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/map">) {
  // 自治体名とタブは (tabs)/layout.tsx の固定のヘッダー。この下をスクロールする
  return (
    <PageSlide>
      <main className="mx-auto flex min-h-0 w-full max-w-md flex-1 flex-col overflow-y-auto overscroll-contain px-4 pt-3">
        <Suspense fallback={<MapSkeleton />}>
          <MapView params={params} searchParams={searchParams} />
        </Suspense>
      </main>
    </PageSlide>
  );
}

async function MapView({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/map">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { tag } = await searchParams;
  // 地図はタグで絞ったものだけ。タグが無ければ、締め切ったトピックの多いタグを開く
  let map = await apiGet<TopicMap>(
    topicMapApi(slug, typeof tag === "string" ? tag : null),
  );
  if (!map) {
    notFound();
  }
  if (!map.tag && map.tags.length > 0) {
    map = (await apiGet<TopicMap>(topicMapApi(slug, map.tags[0].id))) ?? map;
  }
  // タグを変えたら、選んでいた起点を捨てる（key）
  return <TopicMapView key={map.tag ?? ""} slug={slug} map={map} />;
}

function MapSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-3 h-9 w-2/3 rounded-full bg-zinc-100 dark:bg-zinc-900" />
      <div className="aspect-square rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
