// SPDX-License-Identifier: AGPL-3.0-only

import type { ApiErrorBody } from "./types";

/** backend のエラー（`{"error": {"code", "message"}}`） */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

/** 失敗した応答を ApiError にする（本文が読めなければ code は unknown） */
export async function toApiError(res: Response): Promise<ApiError> {
  const data = (await res.json().catch(() => null)) as ApiErrorBody | null;
  return new ApiError(
    res.status,
    data?.error.code ?? "unknown",
    data?.error.message ?? "通信に失敗しました",
  );
}

/** 画面に出すエラーの文。backend の文があればそれを、無ければ `fallback` */
export function errorMessage(e: unknown, fallback: string): string {
  return e instanceof ApiError ? e.message : fallback;
}
