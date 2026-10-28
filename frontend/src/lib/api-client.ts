// SPDX-License-Identifier: AGPL-3.0-only

// Client Component から同一オリジンの /api/* を呼ぶためのヘルパー。
// 参加者 Cookie（HttpOnly）はブラウザが自動で送る。

import { toApiError } from "./api-error";

export { ApiError, errorMessage } from "./api-error";

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
  return (await res.json()) as T;
}
