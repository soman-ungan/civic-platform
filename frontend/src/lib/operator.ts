// SPDX-License-Identifier: AGPL-3.0-only

// 管理画面（/operator）のタブ・URL・表示の決まり

import type { QueueStatement } from "@/lib/api/types/operator";

/** タブ（1つの切り替えに、この順で並べる）。トピックの作業のあとに、意見の確認待ち */
export const OPERATOR_TABS = [
  { key: "drafts", label: "下書き" },
  { key: "scheduled", label: "公開待ち" },
  { key: "published", label: "公開中" },
  { key: "queue", label: "確認待ち" },
] as const;

export type OperatorTab = (typeof OPERATOR_TABS)[number]["key"];

/** /operator?m={slug}&tab={tab} */
export function operatorHref(slug: string, tab: OperatorTab): string {
  return `/operator?m=${encodeURIComponent(slug)}&tab=${tab}`;
}

export const QUEUE_KIND_LABELS: Record<QueueStatement["kind"], string> = {
  seed: "最初の意見",
  citizen: "市民の投稿",
  added: "あとから足した議会の主張・運営者の意見",
};

export const STATEMENT_STATUS_LABELS: Record<string, string> = {
  flagged: "確認待ち",
  pending: "判定待ち",
};

/** ISO8601 → 端末の時刻での「2026年12月1日 23:59」 */
export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString("ja-JP", {
    dateStyle: "long",
    timeStyle: "short",
  });
}

/** ISO8601 → 端末の時刻での日付（<input type="date"> の値。yyyy-mm-dd） */
export function toDateInput(iso: string): string {
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** <input type="date"> の日付の終わり（端末の時刻で 23:59:59）を ISO8601 にする。期限の変更に使う */
export function endOfDateInput(date: string): string {
  return new Date(`${date}T23:59:59`).toISOString();
}
