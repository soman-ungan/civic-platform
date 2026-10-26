// SPDX-License-Identifier: AGPL-3.0-or-later

// Client Component から同一オリジンの /api/* を呼ぶためのヘルパー。
// 参加者 Cookie（HttpOnly）はブラウザが自動で送る。

import type { ApiErrorBody } from "./types";

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

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
    const data = (await res.json().catch(() => null)) as ApiErrorBody | null;
    throw new ApiError(
      res.status,
      data?.error.code ?? "unknown",
      data?.error.message ?? "通信に失敗しました",
    );
  }
  return (await res.json()) as T;
}
