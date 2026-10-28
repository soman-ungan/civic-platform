// SPDX-License-Identifier: AGPL-3.0-only

// Client Component から同一オリジンの /api/* を呼ぶためのヘルパー。
// 参加者 Cookie（HttpOnly）はブラウザが自動で送る。

import { toApiError } from "./error";

export { ApiError, errorMessage } from "./error";

export async function apiFetch<T>(
  path: string,
  init?: { method?: "GET" | "POST" | "PUT"; body?: unknown },
): Promise<T> {
  const res = await fetch(path, {
    method: init?.method ?? "GET",
    headers:
      init?.body === undefined
        ? undefined
        : { "Content-Type": "application/json" },
    body: init?.body === undefined ? undefined : JSON.stringify(init.body),
    cache: "no-store",
  });
  if (!res.ok) {
    throw await toApiError(res);
  }
  // 本文の無い応答（204。ログアウトなど）
  if (res.status === 204) {
    return undefined as T;
  }
  return (await res.json()) as T;
}
