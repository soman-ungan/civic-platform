// SPDX-License-Identifier: AGPL-3.0-only

// AGPL 第13条: 利用者が、いま動いているプログラムのソースを開けるようにする。
// どちらもビルドのときに埋め込む（frontend/Dockerfile の build-arg。公開してよい値）

/** 公開リポジトリの URL */
export const SOURCE_URL = process.env.NEXT_PUBLIC_SOURCE_URL ?? null;

/** いま動いているコミット（イメージを作ったコミット。手元の開発では無い） */
export const GIT_SHA = process.env.NEXT_PUBLIC_GIT_SHA || null;

/** 動いているコミットのソース（コミットが分からなければリポジトリ） */
export function runningSourceUrl(): string | null {
  if (!SOURCE_URL) return null;
  return GIT_SHA ? `${SOURCE_URL}/tree/${GIT_SHA}` : SOURCE_URL;
}
