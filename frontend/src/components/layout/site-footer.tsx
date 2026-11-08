// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { runningSourceUrl } from "@/lib/source";

const LINK =
  "underline underline-offset-2 hover:text-zinc-800 dark:hover:text-zinc-200";

/**
 * 全ページ共通のフッター。AGPL 第13条に基づきソースコードへのリンクを全ページに出す
 * （すべてのページに出るのはフッターだけなので、ここに置く）。
 * 利用規約・プライバシーポリシー・問い合わせは「このサービスについて」から。ログインはヘッダー（auth/header-account.tsx）。
 * 下にスクロールするとたたむ（`.chrome`。layout/chrome-autohide.tsx）。上にスクロールするか、一番下まで来ると出る
 */
export function SiteFooter() {
  // 動いているコミットのソースへ（AGPL 第13条）
  const sourceUrl = runningSourceUrl();

  return (
    <footer className="chrome shrink-0">
      <div className="border-t border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto flex w-full max-w-md items-center justify-between gap-4 px-4 py-2.5 text-xs text-zinc-500">
          <Link href="/about" className={LINK}>
            このサービスについて
          </Link>
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
      </div>
    </footer>
  );
}
