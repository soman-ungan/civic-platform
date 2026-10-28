// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { ConsensusPins } from "@/components/topics/consensus-pins";
import { PageSlide } from "@/components/layout/page-slide";
import { TopicFeed } from "@/components/topics/feed";
import { municipalityApi } from "@/lib/api/paths";
import { apiGet, requestCookie } from "@/lib/api/server";
import type {
  MunicipalityConsensus,
  MunicipalityDetail,
  TopicList,
} from "@/lib/types";

export default function MunicipalityPage({
  params,
  searchParams,
}: PageProps<"/[municipality]">) {
  // 自治体名とタブは (tabs)/layout.tsx の固定のヘッダー。ここでは合意点とタグの列も固定し、
  // トピックのカードの一覧だけをスクロールさせる（TopicFeed）
  return (
    <PageSlide>
      <main className="mx-auto flex min-h-0 w-full max-w-md flex-1 flex-col px-4 pt-3">
        <Suspense fallback={<TimelineSkeleton />}>
          <Timeline params={params} searchParams={searchParams} />
        </Suspense>
      </main>
    </PageSlide>
  );
}

async function Timeline({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { tag } = await searchParams;
  const path = municipalityApi(slug);
  // 参加状態と、合意点（投票を終えたトピックの分だけ見える）は参加者ごとに違うので、Cookie を backend に渡す
  const cookie = await requestCookie();
  const [municipality, list, consensus] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<TopicList>(`${path}/topics`, { cookie }),
    apiGet<MunicipalityConsensus[]>(`${path}/consensus`, { cookie }),
  ]);
  if (!municipality || !list) {
    notFound();
  }

  return (
    <>
      {consensus && consensus.length > 0 && (
        <ConsensusPins slug={municipality.slug} items={consensus} />
      )}
      <TopicFeed
        slug={municipality.slug}
        topics={list.topics}
        tags={list.tags}
        initialTag={typeof tag === "string" ? tag : null}
      />
    </>
  );
}

function TimelineSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-3 h-11 rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
      <div className="mb-4 h-9 w-2/3 rounded-full bg-zinc-100 dark:bg-zinc-900" />
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="mb-3 h-28 rounded-2xl bg-zinc-100 dark:bg-zinc-900"
        />
      ))}
    </div>
  );
}
