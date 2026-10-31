// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { MinutesQuote } from "@/components/ui/quote";
import type { TopicSource } from "@/lib/api/types/topics";

/**
 * 議会での議論（出典）。会議録の発言からの短い引用を、発言ごとの吹き出しで見せる
 * （トピックの説明と区別できるように、上に区切りの線と見出しを置く）。
 * 発言者は公人（首長・議員）なら氏名つきで議員の記録へリンクし、職員は職名だけ（backend で決める）
 */
export function TopicSources({
  slug,
  sources,
}: {
  slug: string;
  sources: TopicSource[];
}) {
  if (sources.length === 0) return null;
  return (
    <section
      aria-labelledby="sources-title"
      className="mt-4 border-t border-zinc-200 pt-4 dark:border-zinc-800"
    >
      <h2 id="sources-title" className="text-sm font-semibold">
        議会での議論
      </h2>
      <ul className="mt-3 flex flex-col gap-4">
        {sources.map((s, i) => (
          <li key={i}>
            <p className="mb-1 text-xs font-medium text-zinc-600 dark:text-zinc-400">
              {s.official_id ? (
                // 公人は議員の記録へ（ADR 0008）
                <Link
                  href={`/${slug}/council/${s.official_id}`}
                  className="underline underline-offset-2"
                >
                  {s.speaker}
                </Link>
              ) : (
                s.speaker
              )}
            </p>
            <MinutesQuote
              quote={s.quote}
              documentTitle={s.document_title}
              heldOn={s.held_on}
              url={s.url}
            />
          </li>
        ))}
      </ul>
      <p className="mt-3 text-xs leading-5 text-zinc-500">
        引用は会議録からの抜粋です。発言の全体は会議録をご覧ください。
      </p>
    </section>
  );
}
