// SPDX-License-Identifier: AGPL-3.0-only

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

/** 居住地属性の短い表記（表の見出し用） */
export const RESIDENCY_SHORT_LABELS: Record<ResidencyType, string> = {
  resident: "住民",
  neighbor: "周辺",
  commuter: "通勤・通学",
  related: "関係人口",
};

/** グループの色（意見マップとカードで共通。並びはグループ A, B, C…） */
export const GROUP_COLORS = [
  "#0284c7",
  "#d97706",
  "#059669",
  "#7c3aed",
  "#e11d48",
];

export function percent(numerator: number, denominator: number): number {
  return denominator === 0 ? 0 : Math.round((numerator / denominator) * 100);
}
