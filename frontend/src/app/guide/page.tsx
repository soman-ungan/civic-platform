// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import { Suspense } from "react";

import { StartButton, StartLink } from "@/components/guide/links";
import { GuideTour } from "@/components/guide/tour";
import { pageMetadata } from "@/lib/metadata";

export const metadata: Metadata = pageMetadata(
  "使い方",
  "架空の例で、投票してみんなの意見を見るまでの流れを試せます。",
);

/**
 * 使い方（チュートリアル）。自治体のトップを初めて開いたときに出す（components/guide/first-visit.tsx）。
 * 例は架空で、本物のデータにも API にも触れない（docs/plans/2026-11-20-tutorial.md）
 */
export default function GuidePage() {
  const start = (
    // ?from= を読むのは Suspense の中（読み込みの前は、トップへのリンク）
    <Suspense fallback={<StartLink href="/" />}>
      <StartButton />
    </Suspense>
  );
  return (
    <main className="min-h-0 flex-1 overflow-y-auto">
      <article className="mx-auto w-full max-w-md px-4 pt-4 pb-10 text-sm leading-7 text-zinc-700 dark:text-zinc-300">
        <h1 className="text-xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
          使い方
        </h1>
        <p className="mt-3">
          まちの議会で話し合われていることに投票して、市民どうしの「合意できる点」を見つける場です。架空の例で、流れを試してみてください。
        </p>
        <div className="mt-4">{start}</div>
        <GuideTour />
        <div className="mt-10">{start}</div>
      </article>
    </main>
  );
}
