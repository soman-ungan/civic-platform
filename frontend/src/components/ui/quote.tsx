// SPDX-License-Identifier: AGPL-3.0-only

import { formatDate } from "@/lib/dates";

/**
 * 会議録からの引用の吹き出しと、その下の出典（会議名・日付。URL があれば会議録へのリンク）。
 * トピック詳細の「議会での議論」と、議員の記録で使う
 */
export function MinutesQuote({
  quote,
  documentTitle,
  heldOn,
  url,
}: {
  quote: string;
  documentTitle: string;
  heldOn: string | null;
  url: string | null;
}) {
  return (
    <>
      <blockquote className="rounded-2xl rounded-tl-sm bg-zinc-100 px-3.5 py-2.5 text-sm leading-6 text-zinc-800 dark:bg-zinc-800 dark:text-zinc-200">
        「{quote}」
      </blockquote>
      <p className="mt-1 text-xs text-zinc-500">
        {url ? (
          <a
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2"
          >
            {documentTitle}
          </a>
        ) : (
          documentTitle
        )}
        {heldOn && `、${formatDate(heldOn)}`}
      </p>
    </>
  );
}
