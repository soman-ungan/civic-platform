// SPDX-License-Identifier: AGPL-3.0-only

import { AccountLinks } from "@/components/account-links";

/** 全ページ共通のフッター。AGPL 第13条に基づきソースコードへのリンクを常に表示する。 */
export function SiteFooter() {
  const sourceUrl = process.env.NEXT_PUBLIC_SOURCE_URL;

  return (
    <footer className="shrink-0 border-t border-zinc-200 dark:border-zinc-800">
      <div className="mx-auto flex w-full max-w-md flex-wrap items-center justify-between gap-x-4 gap-y-1 px-4 py-2.5 text-xs text-zinc-500">
        <p>AGPL-3.0-only で公開しています</p>
        <AccountLinks />
        {sourceUrl && (
          <a
            href={sourceUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2 hover:text-zinc-800 dark:hover:text-zinc-200"
          >
            ソースコード
          </a>
        )}
      </div>
    </footer>
  );
}
