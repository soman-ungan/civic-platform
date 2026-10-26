// SPDX-License-Identifier: AGPL-3.0-or-later

import type { ResidencyType, Topic } from "./types";

export const RESIDENCY_LABELS: Record<ResidencyType, string> = {
  resident: "住民",
  neighbor: "周辺に住んでいる",
  commuter: "通勤・通学している",
  related: "その他の関わりがある（関係人口）",
};

export const SOURCE_LABELS: Record<Topic["source_type"], string> = {
  assembly: "議会",
  citizen: "市民提案",
};

/** 意見の最大文字数（backend の MAX_STATEMENT_CHARS と同じ） */
export const MAX_STATEMENT_CHARS = 140;

/** サーバーと同じく、Unicode のコードポイント単位で数える（絵文字も1文字） */
export function countChars(text: string): number {
  return [...text].length;
}
