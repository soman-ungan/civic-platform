// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { Suspense } from "react";

import { HeaderAccount } from "@/components/auth/header-account";
import { TopicDetailView, TopicSkeleton } from "@/components/topics/detail";
import { apiGet } from "@/lib/api/server";
import type { TopicDetail } from "@/lib/api/types/topics";
import { pageMetadata } from "@/lib/metadata";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import { municipalityApi, topicApi } from "@/lib/api/paths";

/** 共有したときに、トピックの題と説明が出るように */
export async function generateMetadata({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  const { municipality: slug, id } = await params;
  const [m, topic] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<TopicDetail>(topicApi(slug, id)),
  ]);
  if (!m || !topic) {
    return {};
  }
  return pageMetadata(`${topic.title}（${m.name}）`, topic.description);
}

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
  return (
    <TopicDetailView
      slug={slug}
      id={id}
      top={(municipality) => (
        <div className="flex items-center justify-between gap-4">
          <Link
            href={`/${municipality.slug}`}
            className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
          >
            ← {municipality.name}のトピック
          </Link>
          <HeaderAccount />
        </div>
      )}
    />
  );
}
