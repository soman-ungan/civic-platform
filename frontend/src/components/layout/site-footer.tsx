// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { AccountLinks } from "@/components/auth/account-links";
import { runningSourceUrl } from "@/lib/source";

const LINK =
  "underline underline-offset-2 hover:text-zinc-800 dark:hover:text-zinc-200";

/**
 * 全ページ共通のフッター。AGPL 第13条に基づきソースコードへのリンクを常に表示する。
 * 利用規約・プライバシーポリシー・問い合わせは「このサービスについて」から（フッターは常に出るので1行に収める）
 */
export function SiteFooter() {
  // 動いているコミットのソースへ（AGPL 第13条）
  const sourceUrl = runningSourceUrl();

  return (
    <footer className="shrink-0 border-t border-zinc-200 dark:border-zinc-800">
      <div className="mx-auto flex w-full max-w-md flex-wrap items-center justify-between gap-x-4 gap-y-1 px-4 py-2.5 text-xs text-zinc-500">
        <Link href="/about" className={LINK}>
          このサービスについて
        </Link>
        <AccountLinks />
        {sourceUrl && (
          <a
            href={sourceUrl}
            target="_blank"
            rel="noopener noreferrer"
            className={LINK}
          >
            ソースコード（AGPL）
          </a>
        )}
      </div>
    </footer>
  );
}
