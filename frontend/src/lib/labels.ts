// SPDX-License-Identifier: AGPL-3.0-only

import type { ResidencyType } from "@/lib/api/types/participation";
import type { Topic } from "@/lib/api/types/topics";

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

/**
 * トピックの期限の表示（「あと N 日」「締め切りました」）。期限なしなら null。
 * 締め切ったかは backend の `closed` を使う（ルールを写さない）。日数は表示のための目安
 */
export function deadlineLabel(
  topic: Pick<Topic, "closes_at" | "closed">,
): string | null {
  if (topic.closed) return "締め切りました";
  if (!topic.closes_at) return null;
  const days = Math.ceil(
    (Date.parse(topic.closes_at) - Date.now()) / (24 * 60 * 60 * 1000),
  );
  return `あと ${Math.max(days, 1)} 日`;
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

/** 関わり方の並び（住民 → 周辺 → 通勤・通学 → 関係人口）。色の検証はこの並びで行っている */
export const RESIDENCY_ORDER: ResidencyType[] = [
  "resident",
  "neighbor",
  "commuter",
  "related",
];

/** 関わり方の色（globals.css の CSS 変数。ダークモードで切り替わる） */
export const RESIDENCY_COLORS: Record<ResidencyType, string> = {
  resident: "var(--residency-resident)",
  neighbor: "var(--residency-neighbor)",
  commuter: "var(--residency-commuter)",
  related: "var(--residency-related)",
};

/** ドット図での関わり方の形（色を使わず、形で区別する） */
export const RESIDENCY_SHAPES: Record<ResidencyType, string> = {
  resident: "●",
  neighbor: "■",
  commuter: "▲",
  related: "◆",
};
