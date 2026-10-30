// SPDX-License-Identifier: AGPL-3.0-only

// 提言の API の型（backend/src/interface/api/recommendations.rs。中身は records/recommendations.rs。ADR 0005）

import type { ResidencyType } from "./participation";

export type RecommendationStatus = "draft" | "published" | "archived";

/** 一覧の1件 */
export type RecommendationSummary = {
  id: string;
  title: string;
  status: RecommendationStatus;
  created_at: string;
  published_at: string | null;
  /** 合意点の数 */
  consensus: number;
  /** 意見が分かれた点の数 */
  divided: number;
};

/** グループごとの票（提言を作ったときの写し） */
export type GroupVotes = {
  label: string;
  members: number;
  agree: number;
  disagree: number;
  /** 投票した人数（パスを含む） */
  seen: number;
};

/** 合意点か、意見が分かれた点。数字は分析に含まれた人の票 */
export type RecommendationItem = {
  kind: "consensus" | "divided";
  topic_id: string;
  statement_id: string;
  body: string;
  agree: number;
  disagree: number;
  seen: number;
  groups: GroupVotes[];
};

export type RecommendationLink = { id: string; title: string };

/** 元のトピック（作ったときの人数と、関わり方ごとの人数。少人数の区分は null） */
export type RecommendationTopic = RecommendationLink & {
  participants: number;
  residency: Partial<Record<ResidencyType, number | null>>;
};

export type RecommendationDetail = {
  id: string;
  municipality_id: string;
  title: string;
  preface: string;
  status: RecommendationStatus;
  consensus_threshold: number;
  divided_threshold: number;
  created_at: string;
  published_at: string | null;
  topics: RecommendationTopic[];
  /** 子の提言 */
  children: RecommendationLink[];
  /** この提言を子に持つ公開済みの提言 */
  parents: RecommendationLink[];
  /** この提言から作った第2回のトピック（公開済み） */
  second_round: RecommendationLink[];
  items: RecommendationItem[];
};
