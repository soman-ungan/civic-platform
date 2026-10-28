// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { GroupCompositionChart } from "./group-composition";
import { GroupTable } from "./group-table";
import { OpinionMap } from "./opinion-map";
import { ResidencyDotPlot } from "./residency-dot-plot";
import { BUTTON_SECONDARY, ERROR_TEXT } from "@/components/ui/styles";
import { topicApi } from "@/lib/api/paths";
import { useApi } from "@/lib/api/use-api";
import type { AnalysisView } from "@/lib/api/types/analysis";

/**
 * 「みんなの意見」: 意見マップ、合意点、グループと代表意見、居住地属性ごとの違い。
 * 計算は Polis のアルゴリズム（docs/polis-math.md）で、backend が定期的に更新する。
 */
export function AnalysisPanel({
  slug,
  topicId,
  refreshKey,
  initial,
}: {
  slug: string;
  topicId: string;
  /** 値が変わると読み込み直す（分析を見られるようになったときなど） */
  refreshKey: number;
  /** サーバー側で取ってきた最初の分析 */
  initial?: AnalysisView | null;
}) {
  const {
    data: view,
    error,
    reload: load,
  } = useApi<AnalysisView>(
    `${topicApi(slug, topicId)}/analysis`,
    refreshKey,
    initial,
  );
  const [reloading, setReloading] = useState(false);

  async function reload() {
    setReloading(true);
    await load();
    setReloading(false);
  }

  return (
    <section
      aria-labelledby="analysis-title"
      className="mt-10 border-t border-zinc-200 pt-6 dark:border-zinc-800"
    >
      <div className="mb-4 flex items-center justify-between">
        <h2 id="analysis-title" className="text-lg font-bold">
          みんなの意見
        </h2>
        {view?.status !== "locked" && (
          <button
            type="button"
            onClick={() => void reload()}
            disabled={reloading}
            className={`h-9 px-3 text-xs ${BUTTON_SECONDARY}`}
          >
            {reloading ? "更新中…" : "最新の分析を見る"}
          </button>
        )}
      </div>

      {error && <p className={ERROR_TEXT}>分析を読み込めませんでした。</p>}
      {!error && view === null && (
        <div
          aria-hidden
          className="aspect-square animate-pulse rounded-2xl bg-zinc-100 dark:bg-zinc-900"
        />
      )}
      {view?.status === "locked" && (
        <div className="rounded-2xl border border-dashed border-zinc-300 p-5 text-center dark:border-zinc-700">
          <p className="font-semibold">
            あと {view.remaining} 件投票すると、みんなの意見が見られます
          </p>
          <p className="mt-2 text-xs leading-5 text-zinc-500">
            全体の傾向に左右されず、自分の考えで投票してもらうため、議会で出た意見すべてと、合わせて15件（意見がそれより少なければ全部）に投票するまでは表示しません。パスでも投票済みになります。
          </p>
        </div>
      )}
      {view?.status === "pending" && (
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          分析を準備しています。しばらくしてから更新してください。
        </p>
      )}
      {view?.status === "insufficient" && (
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          分析には、もう少し参加者が必要です（{view.participant_count} /{" "}
          {view.required}人）。
        </p>
      )}
      {view?.status === "ready" && <ReadyView view={view} />}
    </section>
  );
}

function ReadyView({
  view,
}: {
  view: Extract<AnalysisView, { status: "ready" }>;
}) {
  return (
    <div className="flex flex-col gap-8">
      <div>
        <OpinionMap map={view.map} groups={view.groups} />
        <p className="mt-2 text-xs leading-5 text-zinc-500">
          投票の傾向が近い人ほど近くに表示されます（{view.analyzed_count}
          人を分析）。
          {view.map.you === null && "投票すると、あなたの位置も表示されます。"}
        </p>
      </div>

      <GroupTable
        groups={view.groups}
        rows={view.statements_by_group}
        myGroupId={view.map.you?.group_id ?? null}
        consensusIds={view.consensus.map((c) => c.statement_id)}
      />

      <GroupCompositionChart
        groups={view.groups}
        composition={view.group_composition}
      />

      <ResidencyDotPlot rows={view.residency} />
    </div>
  );
}
