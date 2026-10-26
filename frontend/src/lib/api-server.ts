// SPDX-License-Identifier: AGPL-3.0-or-later

// Server Component から backend を呼ぶためのヘルパー。
// ブラウザからは同一オリジンの /api/* を呼ぶ（開発時は next.config.ts の rewrites で転送）。

const API_INTERNAL_URL =
  process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080";

/** GET して JSON を返す。404 のときは null（呼び出し側で notFound() にする）。 */
export async function apiGet<T>(path: string): Promise<T | null> {
  const res = await fetch(`${API_INTERNAL_URL}${path}`);
  if (res.status === 404) {
    return null;
  }
  if (!res.ok) {
    throw new Error(`API ${path} が ${res.status} を返しました`);
  }
  return (await res.json()) as T;
}
