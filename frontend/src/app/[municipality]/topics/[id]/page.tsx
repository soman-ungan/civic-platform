// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { OperatorLink } from "@/components/auth/operator-link";
import { TopicInteraction } from "@/components/topics/interaction";
import { TopicFinances } from "@/components/topics/finances";
import { TopicSources } from "@/components/topics/sources";
import { municipalityApi, topicApi } from "@/lib/api/paths";
import { apiGet, requestCookie } from "@/lib/api/server";
import { deadlineLabel, SOURCE_LABELS } from "@/lib/labels";
import type { AnalysisView } from "@/lib/api/types/analysis";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import type { NextStatement } from "@/lib/api/types/participation";
import type { TopicDetail } from "@/lib/api/types/topics";

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
  // 参加者ごとの値（参加状態など）のため、Cookie を backend に渡す
  const cookie = await requestCookie();
  // 最初の意見と分析もサーバー側で取り、HTML に最初から入れる（JS を待ってから取りに行くと、表示が遅れる）。
  // こちらは失敗してもページをエラーにせず、ブラウザ側で取り直す（エラーとやり直しの表示もそちら）
  const [municipality, topic, next, analysis] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<TopicDetail>(topicApi(slug, id), { cookie }),
    apiGet<NextStatement>(`${topicApi(slug, id)}/next-statement`, {
      cookie,
    }).catch(() => null),
    apiGet<AnalysisView>(`${topicApi(slug, id)}/analysis`, { cookie }).catch(
      () => null,
    ),
  ]);
  if (!municipality || !topic) {
    notFound();
  }
  const deadline = deadlineLabel(topic);

  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pb-3">
          <div className="flex items-center justify-between gap-4">
            <Link
              href={`/${municipality.slug}`}
              className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
            >
              ← {municipality.name}のトピック
            </Link>
            <OperatorLink />
          </div>
          <p className="mb-0.5 text-xs text-zinc-500">
            {SOURCE_LABELS[topic.source_type]}
            {topic.session && ` ・ ${topic.session}`}
            {deadline && ` ・ ${deadline}`}
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
        <div className="mx-auto flex w-full max-w-md flex-col px-4 pb-6">
          <TopicInteraction
            slug={municipality.slug}
            topicId={topic.id}
            initialNext={next}
            initialAnalysis={analysis}
            intro={
              topic.description ? (
                <p className="text-sm leading-6 text-zinc-700 dark:text-zinc-300">
                  {topic.description}
                </p>
              ) : undefined
            }
            context={
              <div className="mb-6">
                <TopicSources sources={topic.sources} />
                <TopicFinances
                  slug={municipality.slug}
                  finances={topic.finances}
                />
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
              </div>
            }
          />
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
