// SPDX-License-Identifier: AGPL-3.0-or-later

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { VotingPanel } from "@/components/voting-panel";
import { apiGet } from "@/lib/api-server";
import { SOURCE_LABELS } from "@/lib/labels";
import type { MunicipalityDetail, Topic } from "@/lib/types";

export default function TopicPage({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col px-4 py-6">
      <Suspense fallback={<TopicSkeleton />}>
        <TopicView params={params} />
      </Suspense>
    </main>
  );
}

async function TopicView({
  params,
}: Pick<PageProps<"/[municipality]/topics/[id]">, "params">) {
  const { municipality: slug, id } = await params;
  const path = `/api/municipalities/${encodeURIComponent(slug)}`;
  const [municipality, topic] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<Topic>(`${path}/topics/${encodeURIComponent(id)}`),
  ]);
  if (!municipality || !topic) {
    notFound();
  }

  return (
    <>
      <Link
        href={`/${municipality.slug}`}
        className="mb-4 inline-flex h-11 items-center text-sm text-zinc-600 dark:text-zinc-400"
      >
        ← {municipality.name}のトピック
      </Link>

      <article className="mb-6">
        <p className="mb-1 text-xs text-zinc-500">
          {SOURCE_LABELS[topic.source_type]}
        </p>
        <h1 className="text-xl font-bold leading-snug">{topic.title}</h1>
        {topic.tags.length > 0 && (
          <ul className="mt-2 flex flex-wrap gap-1.5">
            {topic.tags.map((tag) => (
              <li
                key={tag.id}
                className="text-xs text-sky-700 dark:text-sky-400"
              >
                #{tag.name}
              </li>
            ))}
          </ul>
        )}
        {topic.description && (
          <p className="mt-3 text-sm leading-6 text-zinc-700 dark:text-zinc-300">
            {topic.description}
          </p>
        )}
        {topic.source_url && (
          <a
            href={topic.source_url}
            target="_blank"
            rel="noopener noreferrer"
            className="mt-2 inline-block text-sm underline underline-offset-2"
          >
            元の資料を見る
          </a>
        )}
      </article>

      <VotingPanel
        slug={municipality.slug}
        topicId={topic.id}
        districts={municipality.districts}
      />
    </>
  );
}

function TopicSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-6 h-4 w-40 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-6 w-3/4 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-8 h-16 rounded bg-zinc-100 dark:bg-zinc-900" />
      <div className="h-48 rounded-3xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
