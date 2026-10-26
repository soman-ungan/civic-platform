// SPDX-License-Identifier: AGPL-3.0-only

import { notFound } from "next/navigation";
import { Suspense } from "react";

import { TopicFeed } from "@/components/topic-feed";
import { apiGet } from "@/lib/api-server";
import type { MunicipalityDetail, TopicList } from "@/lib/types";

export default function MunicipalityPage({
  params,
  searchParams,
}: PageProps<"/[municipality]">) {
  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col px-4 py-6">
      <Suspense fallback={<TimelineSkeleton />}>
        <Timeline params={params} searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function Timeline({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { tag } = await searchParams;
  const path = `/api/municipalities/${encodeURIComponent(slug)}`;
  const [municipality, list] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<TopicList>(`${path}/topics`),
  ]);
  if (!municipality || !list) {
    notFound();
  }

  return (
    <>
      <header className="mb-4">
        <p className="text-xs text-zinc-500">{municipality.prefecture_name}</p>
        <h1 className="text-2xl font-bold tracking-tight">
          {municipality.name}
        </h1>
      </header>
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
      <div className="mb-2 h-3 w-16 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-6 h-7 w-32 rounded bg-zinc-200 dark:bg-zinc-800" />
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="mb-3 h-28 rounded-2xl bg-zinc-100 dark:bg-zinc-900"
        />
      ))}
    </div>
  );
}
