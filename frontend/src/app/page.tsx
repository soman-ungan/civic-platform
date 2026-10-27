// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { Suspense } from "react";

import { apiGet } from "@/lib/api-server";
import type { Municipality } from "@/lib/types";

// 公開中の自治体が1つだけのときは src/proxy.ts がそのページへリダイレクトする。
// ここに来るのは 0 件または複数件のとき（自治体選択の UI はフェーズ8「展開」で作り込む）。
export default function Home() {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto">
      <div className="mx-auto flex w-full max-w-md flex-col px-4 py-8">
        <header className="mb-6">
          <h1 className="text-2xl font-bold tracking-tight">
            市民参加プラットフォーム
          </h1>
          <p className="mt-2 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
            まちの議事録・予算・決算をわかりやすく見せ、みんなの合意点を見つける場所です。
          </p>
        </header>
        <Suspense
          fallback={<p className="text-sm text-zinc-500">読み込み中…</p>}
        >
          <MunicipalityList />
        </Suspense>
      </div>
    </main>
  );
}

async function MunicipalityList() {
  const municipalities =
    (await apiGet<Municipality[]>("/api/municipalities")) ?? [];
  if (municipalities.length === 0) {
    return (
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        公開中の自治体はまだありません。
      </p>
    );
  }
  return (
    <ul className="flex flex-col gap-3">
      {municipalities.map((m) => (
        <li key={m.id}>
          <Link
            href={`/${m.slug}`}
            className="block rounded-2xl border border-zinc-200 bg-white p-4 shadow-sm active:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900"
          >
            <span className="text-xs text-zinc-500">{m.prefecture_name}</span>
            <span className="block text-lg font-semibold">{m.name}</span>
          </Link>
        </li>
      ))}
    </ul>
  );
}
