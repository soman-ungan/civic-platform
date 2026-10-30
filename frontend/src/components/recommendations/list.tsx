// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { EmptyState } from "@/components/ui/empty-state";
import type { RecommendationSummary } from "@/lib/api/types/recommendations";
import { formatDate } from "@/lib/recommendations";

/** 公開済みの提言の一覧（公開の新しい順。backend の並び） */
export function RecommendationList({
  slug,
  items,
}: {
  slug: string;
  items: RecommendationSummary[];
}) {
  return (
    <div className="-mx-4 min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 pb-4">
      <p className="mb-3 text-xs leading-5 text-zinc-500">
        締め切ったトピックの投票から、どのグループでも賛成が多かった意見（合意点）と、グループによって賛否が分かれた意見を、原文と数字のまま集めたものです。陳情やパブリックコメント、市長への手紙などに使えます。
      </p>
      {items.length === 0 ? (
        <EmptyState>
          まだ提言はありません。トピックが締め切られると、運営者がまとめて公開します。
        </EmptyState>
      ) : (
        <ul className="flex flex-col gap-3">
          {items.map((r) => (
            <li key={r.id}>
              <Link
                href={`/${slug}/recommendations/${r.id}`}
                className="block rounded-2xl border border-zinc-200 p-4 shadow-sm dark:border-zinc-800"
              >
                {r.published_at && (
                  <p className="mb-1 text-xs text-zinc-500">
                    {formatDate(r.published_at)}
                  </p>
                )}
                <h2 className="leading-snug font-semibold">{r.title}</h2>
                <p className="mt-2 text-xs text-zinc-500">
                  合意点 {r.consensus} 件 ・ 意見が分かれた点 {r.divided} 件
                </p>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
