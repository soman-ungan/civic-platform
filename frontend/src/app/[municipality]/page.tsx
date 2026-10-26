// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { TopicFeed } from "@/components/topic-feed";
import { apiGet } from "@/lib/api-server";
import type {
  MunicipalityConsensus,
  MunicipalityDetail,
  TopicList,
} from "@/lib/types";

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
  const [municipality, list, consensus] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<TopicList>(`${path}/topics`),
    apiGet<MunicipalityConsensus[]>(`${path}/consensus`),
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

/** タイムライン最上位の「合意点」。全グループの8割以上が賛成している意見 */
function ConsensusPins({
  slug,
  items,
}: {
  slug: string;
  items: MunicipalityConsensus[];
}) {
  return (
    <section
      aria-labelledby="pins-title"
      className="mb-5 rounded-2xl bg-emerald-50 p-4 dark:bg-emerald-950/40"
    >
      <h2
        id="pins-title"
        className="text-sm font-bold text-emerald-900 dark:text-emerald-200"
      >
        <span aria-hidden>📌 </span>
        みんなの合意点
      </h2>
      <p className="mb-3 text-xs text-emerald-800 dark:text-emerald-300">
        意見の違うグループのどれでも、8割以上が賛成している意見です。
      </p>
      <ul className="flex flex-col gap-3">
        {items.map((c) => (
          <li key={c.statement_id}>
            <Link href={`/${slug}/topics/${c.topic_id}`} className="block">
              <p className="leading-7">{c.body}</p>
              <p className="text-xs text-emerald-800 underline underline-offset-2 dark:text-emerald-300">
                {c.topic_title}
              </p>
            </Link>
          </li>
        ))}
      </ul>
    </section>
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
