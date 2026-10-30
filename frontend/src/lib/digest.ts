// SPDX-License-Identifier: AGPL-3.0-only

// 提言の表示の決まり（ADR 0005）

/** 公開した日（日本時間の「2026年10月30日」） */
export function formatDate(iso: string): string {
  return new Intl.DateTimeFormat("ja-JP", {
    timeZone: "Asia/Tokyo",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(new Date(iso));
}

/** 基準の割合の表記（0.6 → 「6割」） */
export function ratioLabel(value: number): string {
  return `${Math.round(value * 10)}割`;
}
