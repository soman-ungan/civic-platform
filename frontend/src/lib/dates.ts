// SPDX-License-Identifier: AGPL-3.0-only

// 日付の表記（画面をまたいで同じ書き方にする）

/**
 * 日付（日本時間の「2026年10月30日」）。ISO8601 の日時でも、日付だけ（「2026-06-10」。UTC の0時として読み、日本時間でも同じ日）でもよい
 */
export function formatDate(iso: string): string {
  return new Intl.DateTimeFormat("ja-JP", {
    timeZone: "Asia/Tokyo",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(new Date(iso));
}
