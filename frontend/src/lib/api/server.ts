// SPDX-License-Identifier: AGPL-3.0-only

// Server Component から backend を呼ぶためのヘルパー。
// ブラウザからは同一オリジンの /api/* を呼ぶ（開発時は next.config.ts の rewrites で転送）。

import { headers } from "next/headers";

import { apiInternalUrl } from "./base";
import { toApiError } from "./error";

/**
 * GET して JSON を返す。404 のときは null（呼び出し側で notFound() にする）。ほかの失敗は ApiError を投げる。
 * `cookie` を渡すと backend に転送する（参加状態など、参加者ごとの値が必要なとき。`requestCookie()`）
 */
export async function apiGet<T>(
  path: string,
  options?: { cookie?: string | null },
): Promise<T | null> {
  const res = await fetch(`${apiInternalUrl()}${path}`, {
    headers: options?.cookie ? { cookie: options.cookie } : undefined,
  });
  if (res.status === 404) {
    return null;
  }
  if (!res.ok) {
    throw await toApiError(res);
  }
  return (await res.json()) as T;
}

/** ブラウザから届いた Cookie（backend に参加者を伝えるため、そのまま渡す） */
export async function requestCookie(): Promise<string | null> {
  return (await headers()).get("cookie");
}
