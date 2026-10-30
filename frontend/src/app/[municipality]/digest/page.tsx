// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { DigestView } from "@/components/digest/view";
import { EmptyState } from "@/components/ui/empty-state";
import { ApiError } from "@/lib/api/error";
import { digestApi, municipalityApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { Digest } from "@/lib/api/types/digest";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import { idList } from "@/lib/digest";

/** 市民のまとめ（?anchor=起点&topics=近傍から選んだトピック）。保存しないので、URL で共有・印刷できる */
export default function DigestPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/digest">) {
  // 見出しは固定のヘッダーにし、中身だけをスクロールする。印刷のときは全体を1枚の文書として流す
  return (
    <main className="flex min-h-0 flex-1 flex-col print:block">
      <Suspense fallback={<Skeleton />}>
        <DigestPageView params={params} searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function DigestPageView({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/digest">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { anchor, topics } = await searchParams;
  const municipality = await apiGet<MunicipalityDetail>(municipalityApi(slug));
  if (!municipality || typeof anchor !== "string") {
    notFound();
  }
  // 近傍の外のトピックなどが URL に入っていたら（422）、その理由を出す
  let digest: Digest | null = null;
  let problem: string | null = null;
  try {
    digest = await apiGet<Digest>(digestApi(slug, anchor, idList(topics)));
  } catch (e) {
    if (e instanceof ApiError && e.status === 422) problem = e.message;
    else throw e;
  }

  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800 print:border-0">
        <div className="mx-auto w-full max-w-md px-4 pb-3 print:max-w-none">
          <Link
            href={`/${municipality.slug}/map`}
            className="inline-flex h-10 items-center text-sm text-zinc-600 print:hidden dark:text-zinc-400"
          >
            ← これまでの議論
          </Link>
          <p className="mb-0.5 text-xs text-zinc-500">
            {municipality.name}の市民の投票のまとめ
          </p>
          <h1 className="text-lg leading-snug font-bold">
            {digest?.topics[0]?.title ?? "まとめ"}
            {digest && digest.topics.length > 1 && (
              <span className="text-sm font-normal text-zinc-500">
                {" "}
                ほか{digest.topics.length - 1}件
              </span>
            )}
          </h1>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain print:overflow-visible">
        <div className="mx-auto flex w-full max-w-md flex-col gap-8 px-4 pt-4 print:max-w-none">
          {digest ? (
            <DigestView slug={municipality.slug} digest={digest} />
          ) : (
            <EmptyState>{problem ?? "まとめを作れませんでした"}</EmptyState>
          )}
        </div>
      </div>
    </>
  );
}

function Skeleton() {
  return (
    <div
      aria-hidden
      className="mx-auto w-full max-w-md animate-pulse px-4 py-3"
    >
      <div className="mb-4 h-4 w-40 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-6 w-3/4 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="h-48 rounded-3xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
