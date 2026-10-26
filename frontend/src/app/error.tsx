// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useEffect } from "react";

export default function Error({
  error,
  retry,
}: {
  error: unknown;
  retry: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col px-4 py-12">
      <h1 className="text-xl font-bold">読み込みに失敗しました</h1>
      <p className="mt-2 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
        通信状態を確認して、もう一度お試しください。
      </p>
      <button
        type="button"
        onClick={() => retry()}
        className="mt-6 h-11 rounded-full bg-zinc-900 px-5 text-sm font-medium text-white dark:bg-zinc-100 dark:text-zinc-900"
      >
        再読み込み
      </button>
    </main>
  );
}
