// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { EmptyState } from "@/components/ui/empty-state";
import type { MunicipalityConsensus } from "@/lib/api/types/analysis";
import { consensusMapHref } from "@/lib/digest";

/**
 * 合意点の一覧（全グループの8割以上が賛成している意見）。
 * 元のトピックが締め切ったものなら、押すと同じページの地図でそのトピックを起点にして示す。受付中ならトピックへ
 */
export function ConsensusList({
  slug,
  items,
}: {
  slug: string;
  items: MunicipalityConsensus[];
}) {
  if (items.length === 0) {
    return (
      <EmptyState>
        まだ合意点はありません。トピックの最初の意見すべてに投票すると、そのトピックの合意点がここに出ます。
      </EmptyState>
    );
  }
  return (
    <ul className="flex flex-col gap-3">
      {items.map((c) => (
        <li key={c.statement_id}>
          <Link
            href={
              c.closed
                ? consensusMapHref(slug, { focus: c.topic_id })
                : `/${slug}/topics/${c.topic_id}`
            }
            scroll={!c.closed}
            className="block rounded-2xl bg-emerald-50 p-4 dark:bg-emerald-950/40"
          >
            <p className="leading-7">{c.body}</p>
            <p className="mt-1 flex items-baseline justify-between gap-2 text-xs text-emerald-800 dark:text-emerald-300">
              <span className="min-w-0 underline underline-offset-2">
                {c.topic_title}
              </span>
              <span className="shrink-0">
                {c.closed ? "地図で見る ↓" : "トピックへ →"}
              </span>
            </p>
          </Link>
        </li>
      ))}
    </ul>
  );
}
