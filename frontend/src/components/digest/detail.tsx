// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";

import { DigestView } from "@/components/digest/view";
import { EmptyState } from "@/components/ui/empty-state";
import { ApiError } from "@/lib/api/error";
import { digestApi, municipalityApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { Digest } from "@/lib/api/types/digest";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import { consensusMapHref, digestHref, idList } from "@/lib/digest";

/**
 * 市民のまとめの中身（固定のヘッダーと、スクロールする本体）。
 * 普通のまとめのページと、合意点のページの上に重ねるレイヤ（`consensus/@sheet/(..)digest`）の両方で使う。
 * `inSheet` のときは戻るリンクを出さず、印刷は普通のページを開くリンクにする
 */
export async function DigestDetailView({
  slug,
  anchor,
  topics,
  inSheet = false,
}: {
  slug: string;
  anchor: string | string[] | undefined;
  topics: string | string[] | undefined;
  inSheet?: boolean;
}) {
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
      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain print:overflow-visible">
        <div className="mx-auto flex w-full max-w-md flex-col gap-8 px-4 print:max-w-none">
          {/* まとめの見出しは、スクロールすると流れる（トピックの題と説明を上に残すため） */}
          <header className="-mx-4 border-b border-zinc-200 dark:border-zinc-800 print:border-0">
            <div className="px-4 pb-3">
              {!inSheet && (
                <Link
                  href={consensusMapHref(municipality.slug, {
                    focus: digest?.anchor ?? anchor,
                  })}
                  className="inline-flex h-10 items-center text-sm text-zinc-600 print:hidden dark:text-zinc-400"
                >
                  ← みんなの合意点
                </Link>
              )}
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
          {digest ? (
            <DigestView
              slug={municipality.slug}
              digest={digest}
              printHref={
                inSheet
                  ? digestHref(municipality.slug, anchor, idList(topics))
                  : undefined
              }
            />
          ) : (
            <EmptyState>{problem ?? "まとめを作れませんでした"}</EmptyState>
          )}
        </div>
      </div>
    </>
  );
}

export function DigestSkeleton() {
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
