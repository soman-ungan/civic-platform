// SPDX-License-Identifier: AGPL-3.0-only

/**
 * サーバー側から backend を呼ぶ URL（ブラウザは同一オリジンの /api/* を呼ぶ）。
 * next.config.ts からも使うので、読み込んだ時ではなく呼んだ時に環境変数を読む（.env を読んだあとに呼ばれるように）
 */
export function apiInternalUrl(): string {
  return process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080";
}
