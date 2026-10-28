// SPDX-License-Identifier: AGPL-3.0-only

import type { TopicSource } from "@/lib/api/types/topics";

/** 2025-09-05 → 2025年9月5日 */
function formatDate(isoDate: string): string {
  const [y, m, d] = isoDate.split("-").map(Number);
  return `${y}年${m}月${d}日`;
}

/**
 * 議会での議論（出典）。会議録の発言からの短い引用を、発言ごとの吹き出しで見せる
 * （トピックの説明と区別できるように、上に区切りの線と見出しを置く）。
 * 発言者は公人（首長・議員）なら氏名つき、職員は職名だけ（backend で決める）
 */
export function TopicSources({ sources }: { sources: TopicSource[] }) {
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
              {s.speaker}
            </p>
            <blockquote className="relative rounded-2xl rounded-tl-sm bg-zinc-100 px-3.5 py-2.5 text-sm leading-6 text-zinc-800 dark:bg-zinc-800 dark:text-zinc-200">
              「{s.quote}」
            </blockquote>
            <p className="mt-1 text-xs text-zinc-500">
              {s.url ? (
                <a
                  href={s.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="underline underline-offset-2"
                >
                  {s.document_title}
                </a>
              ) : (
                s.document_title
              )}
              {s.held_on && `、${formatDate(s.held_on)}`}
            </p>
          </li>
        ))}
      </ul>
      <p className="mt-3 text-xs leading-5 text-zinc-500">
        引用は会議録からの抜粋です。発言の全体は会議録をご覧ください。
      </p>
    </section>
  );
}
