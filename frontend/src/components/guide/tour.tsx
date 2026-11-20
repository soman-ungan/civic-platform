// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { AnalysisReady } from "@/components/analysis/panel";
import { STICKY_TITLE } from "@/components/analysis/sticky";
import { VOTE_BUTTONS } from "@/components/participation/vote-buttons";
import type { VoteValue } from "@/lib/api/types/participation";

import { EXAMPLE } from "./example";

/**
 * 使い方: 架空の例題で、トピック → 投票 → みんなの意見 の流れを試す。
 * 票は画面の中だけに持ち、どこにも送らない。例の分析は固定のデータ（example.ts）を、本物の分析と同じ部品で描く
 */
export function GuideTour() {
  const [votes, setVotes] = useState<VoteValue[]>([]);
  const [skipped, setSkipped] = useState(false);
  const total = EXAMPLE.statements.length;
  const current = EXAMPLE.statements[votes.length];
  const unlocked = skipped || votes.length >= total;

  return (
    <>
      <Step n={1} title="議会の論点がトピックになります">
        <p>
          議会で話し合われたことを「トピック」として出します。最初に並ぶ意見は、議会で実際に出た主張です。市民からトピックを提案することもできます。
        </p>
        <div className="mt-3 rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
          <p className="font-bold text-zinc-900 dark:text-zinc-100">
            {EXAMPLE.title}
          </p>
          <p className="mt-1 text-xs leading-5 text-zinc-500">
            {EXAMPLE.description}
          </p>
        </div>
      </Step>

      <Step n={2} title="意見に投票します">
        <p>
          意見を1つずつ読んで「賛成・反対・パス」で答えます。読むことと投票は、ログインなしでできます。試しに、例の意見に投票してみてください（この画面の票はどこにも送りません）。
        </p>
        <div className="mt-3 mb-3 flex items-center gap-3 text-xs text-zinc-500">
          <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
            <div
              className="h-full rounded-full bg-zinc-900 transition-[width] dark:bg-zinc-100"
              style={{ width: `${Math.round((votes.length / total) * 100)}%` }}
            />
          </div>
          <span>
            {votes.length} / {total}
          </span>
        </div>
        {current ? (
          <>
            <div aria-live="polite">
              <blockquote
                key={current.id}
                className="flex min-h-36 items-center rounded-3xl border border-zinc-200 bg-white p-6 text-lg leading-8 text-zinc-900 shadow-sm motion-safe:animate-card-in dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-100"
              >
                {current.body}
              </blockquote>
            </div>
            <div className="mt-4 grid grid-cols-3 gap-2">
              {VOTE_BUTTONS.map((b) => (
                <button
                  key={b.value}
                  type="button"
                  onClick={() => setVotes((v) => [...v, b.value])}
                  className={`h-14 rounded-2xl text-base font-semibold ${b.className}`}
                >
                  {b.label}
                </button>
              ))}
            </div>
          </>
        ) : (
          <p className="rounded-3xl border border-zinc-200 p-6 text-center font-semibold text-zinc-900 dark:border-zinc-800 dark:text-zinc-100">
            最初の意見すべてに投票しました
          </p>
        )}
      </Step>

      <Step n={3} title="みんなの意見が見られます">
        <p>
          最初の意見すべてに投票すると、「みんなの意見」が開きます。投票の傾向が近い人どうしのグループと、グループをこえて賛成が多い「合意点」が分かります。まわりの傾向に左右されず自分の考えで投票してもらうため、投票するまでは見られません。
        </p>
        {unlocked ? (
          <div className="mt-6">
            <div className={`mb-2 ${STICKY_TITLE}`}>
              <h3 className="text-lg font-bold text-zinc-900 dark:text-zinc-100">
                みんなの意見（例）
              </h3>
            </div>
            <p className="mb-4 rounded-xl bg-amber-50 px-3 py-2 text-xs leading-5 text-amber-900 dark:bg-amber-950 dark:text-amber-200">
              架空の例です。{EXAMPLE.analysis.participant_count}
              人分の架空の投票を、実際のトピックと同じ計算で分析しました。実際の投票ではありません。
            </p>
            <AnalysisReady view={EXAMPLE.analysis} />
          </div>
        ) : (
          <div className="mt-3 rounded-2xl border border-dashed border-zinc-300 p-5 text-center dark:border-zinc-700">
            <p className="font-semibold text-zinc-900 dark:text-zinc-100">
              あと {total - votes.length} 件投票すると、みんなの意見が見られます
            </p>
            <button
              type="button"
              onClick={() => setSkipped(true)}
              className="mt-2 text-xs text-zinc-500 underline underline-offset-2"
            >
              投票せずに例を見る
            </button>
          </div>
        )}
      </Step>

      <Step n={4} title="意見を出す・トピックを提案する">
        <ul className="list-disc space-y-1 pl-5">
          <li>
            分析が見られるようになったら、自分の意見を1件出せます。ほかの人がその意見に投票します
          </li>
          <li>まちで話し合いたいことを、新しいトピックとして提案できます</li>
          <li>
            意見を出す・提案するには、パスキー（指紋や顔での確認）を作ります。名前やメールアドレスは要りません
          </li>
          <li>
            これまでの合意点は、トピックの一覧の上の「みんなの合意点」でまとめて見られます
          </li>
        </ul>
        <p className="mt-3">
          年齢・性別・住んでいる地区は聞きません。人を属性で分けず、意見の重なりを見るためです。
        </p>
      </Step>
    </>
  );
}

function Step({
  n,
  title,
  children,
}: {
  n: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mt-8">
      <h2 className="flex items-center gap-2 text-base font-bold text-zinc-900 dark:text-zinc-100">
        <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-zinc-900 text-xs text-white dark:bg-zinc-100 dark:text-zinc-900">
          {n}
        </span>
        {title}
      </h2>
      <div className="mt-2">{children}</div>
    </section>
  );
}
