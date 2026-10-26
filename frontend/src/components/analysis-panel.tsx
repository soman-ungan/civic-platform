// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect, useState } from "react";

import { GroupTable } from "@/components/group-table";
import { OpinionMap } from "@/components/opinion-map";
import { apiFetch } from "@/lib/api-client";
import { RESIDENCY_SHORT_LABELS, percent } from "@/lib/labels";
import type { AnalysisView, ResidencyRow, ResidencyType } from "@/lib/types";

const RESIDENCY_ORDER: ResidencyType[] = [
  "resident",
  "neighbor",
  "commuter",
  "related",
];

/**
 * 「みんなの意見」: 意見マップ、合意点、グループと代表意見、居住地属性ごとの違い。
 * 計算は Polis のアルゴリズム（docs/polis-math.md）で、backend が定期的に更新する。
 */
export function AnalysisPanel({
  slug,
  topicId,
}: {
  slug: string;
  topicId: string;
}) {
  const endpoint = `/api/municipalities/${encodeURIComponent(slug)}/topics/${encodeURIComponent(topicId)}/analysis`;
  const [view, setView] = useState<AnalysisView | null>(null);
  const [error, setError] = useState(false);
  const [reloading, setReloading] = useState(false);

  useEffect(() => {
    let ignore = false;
    apiFetch<AnalysisView>(endpoint)
      .then((v) => {
        if (!ignore) setView(v);
      })
      .catch(() => {
        if (!ignore) setError(true);
      });
    return () => {
      ignore = true;
    };
  }, [endpoint]);

  async function reload() {
    setReloading(true);
    try {
      setView(await apiFetch<AnalysisView>(endpoint));
      setError(false);
    } catch {
      setError(true);
    } finally {
      setReloading(false);
    }
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
        <button
          type="button"
          onClick={() => void reload()}
          disabled={reloading}
          className="h-9 rounded-full border border-zinc-300 px-3 text-xs disabled:opacity-50 dark:border-zinc-700"
        >
          {reloading ? "更新中…" : "最新の分析を見る"}
        </button>
      </div>

      {error && (
        <p className="text-sm text-rose-600">分析を読み込めませんでした。</p>
      )}
      {!error && view === null && (
        <div
          aria-hidden
          className="aspect-square animate-pulse rounded-2xl bg-zinc-100 dark:bg-zinc-900"
        />
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

      <div>
        <h3 className="mb-2 font-semibold">合意点</h3>
        {view.consensus.length === 0 ? (
          <p className="text-sm text-zinc-600 dark:text-zinc-400">
            まだ、すべてのグループの8割以上が賛成している意見はありません。
          </p>
        ) : (
          <ul className="flex flex-col gap-2">
            {view.consensus.map((c) => (
              <li
                key={c.statement_id}
                className="rounded-2xl bg-emerald-50 p-4 dark:bg-emerald-950/40"
              >
                <p className="leading-7">{c.body}</p>
                <p className="mt-1 text-xs text-emerald-800 dark:text-emerald-300">
                  {c.groups
                    .map(
                      (g) => `グループ${g.label}: ${percent(g.agree, g.seen)}%`,
                    )
                    .join(" ・ ")}{" "}
                  が賛成
                </p>
              </li>
            ))}
          </ul>
        )}
      </div>

      <GroupTable
        groups={view.groups}
        rows={view.statements_by_group}
        myGroupId={view.map.you?.group_id ?? null}
      />

      <ResidencyTable rows={view.residency} />
    </div>
  );
}

/** 居住地属性ごとの賛成率。差が大きい意見を上から5件（人数が少ないセルは「—」） */
function ResidencyTable({ rows }: { rows: ResidencyRow[] }) {
  const comparable = rows
    .map((row) => {
      const rates = RESIDENCY_ORDER.flatMap((t) => {
        const cell = row.by_type[t];
        return cell ? [cell.agree / cell.seen] : [];
      });
      return {
        row,
        spread:
          rates.length >= 2 ? Math.max(...rates) - Math.min(...rates) : -1,
      };
    })
    .filter((x) => x.spread >= 0)
    .sort((a, b) => b.spread - a.spread)
    .slice(0, 5);

  return (
    <div>
      <h3 className="mb-1 font-semibold">住民と関係人口の違い</h3>
      <p className="mb-2 text-xs text-zinc-500">
        賛成した人の割合。5人未満の区分は表示しません。
      </p>
      {comparable.length === 0 ? (
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          比べられるだけの人数が、まだ集まっていません。
        </p>
      ) : (
        <div className="-mx-4 overflow-x-auto px-4">
          <table className="w-full min-w-[22rem] text-left text-xs">
            <thead>
              <tr className="text-zinc-500">
                <th className="py-2 pr-2 font-normal">意見</th>
                {RESIDENCY_ORDER.map((t) => (
                  <th
                    key={t}
                    className="px-1 py-2 text-right font-normal whitespace-nowrap"
                  >
                    {RESIDENCY_SHORT_LABELS[t]}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {comparable.map(({ row }) => (
                <tr
                  key={row.statement_id}
                  className="border-t border-zinc-200 align-top dark:border-zinc-800"
                >
                  <td className="py-2 pr-2 leading-5">{row.body}</td>
                  {RESIDENCY_ORDER.map((t) => {
                    const cell = row.by_type[t];
                    return (
                      <td
                        key={t}
                        className="px-1 py-2 text-right whitespace-nowrap"
                      >
                        {cell ? `${percent(cell.agree, cell.seen)}%` : "—"}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
