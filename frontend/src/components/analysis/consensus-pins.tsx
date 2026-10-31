// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

/**
 * タイムラインの上の「みんなの合意点」: 合意点のページ（合意点の一覧と、締め切ったトピックの地図）への入口。
 * 件数は、この人が投票し終えたトピックの合意点の数（ページでは締め切ったトピックの合意点も出る）
 */
export function ConsensusPins({
  slug,
  count,
}: {
  slug: string;
  count: number;
}) {
  return (
    <Link
      href={`/${slug}/consensus`}
      className="mb-3 flex min-h-11 shrink-0 items-center justify-between gap-2 rounded-2xl bg-emerald-50 px-4 text-sm font-bold text-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-200"
    >
      <span>
        <span aria-hidden>📌 </span>
        みんなの合意点
        {count > 0 && `（${count}件）`}
        <span className="ml-1 text-xs font-normal">・これまでの議論</span>
      </span>
      <span aria-hidden className="text-xs">
        →
      </span>
    </Link>
  );
}
