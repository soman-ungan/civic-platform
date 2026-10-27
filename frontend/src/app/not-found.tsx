// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

export default function NotFound() {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto">
      <div className="mx-auto flex w-full max-w-md flex-col px-4 py-12">
        <h1 className="text-xl font-bold">ページが見つかりません</h1>
        <p className="mt-2 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
          URL が間違っているか、公開が終了した可能性があります。
        </p>
        <Link
          href="/"
          className="mt-6 text-sm font-medium underline underline-offset-2"
        >
          トップへ戻る
        </Link>
      </div>
    </main>
  );
}
