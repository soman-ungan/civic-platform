// SPDX-License-Identifier: AGPL-3.0-only

// Server Component から backend を呼ぶためのヘルパー。
// ブラウザからは同一オリジンの /api/* を呼ぶ（開発時は next.config.ts の rewrites で転送）。

const API_INTERNAL_URL =
  process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080";

/**
 * GET して JSON を返す。404 のときは null（呼び出し側で notFound() にする）。
 * `cookie` を渡すと backend に転送する（参加状態など、参加者ごとの値が必要なとき）。
 */
export async function apiGet<T>(
  path: string,
  options?: { cookie?: string | null },
): Promise<T | null> {
  const res = await fetch(`${API_INTERNAL_URL}${path}`, {
    headers: options?.cookie ? { cookie: options.cookie } : undefined,
  });
  if (res.status === 404) {
    return null;
  }
  if (!res.ok) {
    throw new Error(`API ${path} が ${res.status} を返しました`);
  }
  return (await res.json()) as T;
}
