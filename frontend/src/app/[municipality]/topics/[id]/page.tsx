// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { TopicInteraction } from "@/components/topic-interaction";
import { TopicFinances } from "@/components/topic-finances";
import { TopicSources } from "@/components/topic-sources";
import { apiGet } from "@/lib/api-server";
import { SOURCE_LABELS } from "@/lib/labels";
import type { MunicipalityDetail, TopicDetail } from "@/lib/types";

export default function TopicPage({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  // トピックのタイトルとタグは固定のヘッダーにし、説明・投票・みんなの意見の部分だけをスクロールする
  return (
    <main className="flex min-h-0 flex-1 flex-col">
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
    apiGet<TopicDetail>(`${path}/topics/${encodeURIComponent(id)}`),
  ]);
  if (!municipality || !topic) {
    notFound();
  }

  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pb-3">
          <Link
            href={`/${municipality.slug}`}
            className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
          >
            ← {municipality.name}のトピック
          </Link>
          <p className="mb-0.5 text-xs text-zinc-500">
            {SOURCE_LABELS[topic.source_type]}
          </p>
          <h1 className="text-lg leading-snug font-bold">{topic.title}</h1>
          {topic.tags.length > 0 && (
            <ul className="-mx-4 mt-1.5 flex gap-2 overflow-x-auto px-4 whitespace-nowrap [scrollbar-width:none]">
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
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto flex w-full max-w-md flex-col px-4 pt-3 pb-6">
          <article className="mb-6">
            {topic.description && (
              <p className="text-sm leading-6 text-zinc-700 dark:text-zinc-300">
                {topic.description}
              </p>
            )}
            <TopicSources sources={topic.sources} />
            <TopicFinances slug={municipality.slug} finances={topic.finances} />
            {topic.source_url && topic.sources.length === 0 && (
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

          <TopicInteraction slug={municipality.slug} topicId={topic.id} />
        </div>
      </div>
    </>
  );
}

function TopicSkeleton() {
  return (
    <div
      aria-hidden
      className="mx-auto w-full max-w-md animate-pulse px-4 py-3"
    >
      <div className="mb-4 h-4 w-40 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-6 w-3/4 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-8 h-16 rounded bg-zinc-100 dark:bg-zinc-900" />
      <div className="h-48 rounded-3xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
