// SPDX-License-Identifier: AGPL-3.0-only

// トピックマップと市民のまとめの API の型（backend/src/interface/api/digest.rs。ADR 0005）

import type { ResidencyType } from "./participation";

/** 地図で絞れるタグ（締め切ったトピックに付いているもの） */
export type MapTag = {
  id: string;
  kind: "field" | "district";
  name: string;
  /** そのタグの付いた締め切ったトピックの数 */
  count: number;
};

/** 地図の点（トピック1件）。座標は -1〜1 */
export type MapPoint = {
  id: string;
  title: string;
  closes_at: string;
  participants: number;
  x: number;
  y: number;
};

/** 似ている組（地図の線。起点の近傍の判定にも使う） */
export type MapEdge = { a: string; b: string; score: number };

export type TopicMap = {
  tags: MapTag[];
  /** 選んだタグ（無ければ点も線も空） */
  tag: string | null;
  points: MapPoint[];
  edges: MapEdge[];
};

export type GroupVotes = {
  label: string;
  members: number;
  agree: number;
  disagree: number;
  /** 投票した人数（パスを含む） */
  seen: number;
};

/** 合意点か、意見が分かれた点。数字は分析に含まれた人の票 */
export type DigestItem = {
  kind: "consensus" | "divided";
  statement_id: string;
  body: string;
  agree: number;
  disagree: number;
  seen: number;
  groups: GroupVotes[];
};

export type DigestGroup = {
  label: string;
  members: number;
  /** グループの代表的な意見（Polis の repness） */
  representative: {
    statement_id: string;
    body: string;
    repful_for: "agree" | "disagree";
  }[];
};

export type TopicDigest = {
  id: string;
  title: string;
  description: string;
  closes_at: string;
  /** 起点との似ている度合い（起点は 1） */
  score: number;
  /** 分析の対象の人数 */
  analyzed: number;
  /** 5人以上で、中身を出せるか */
  sufficient: boolean;
  /** 関わり方ごとの人数（少人数の区分は null） */
  residency: Partial<Record<ResidencyType, number | null>>;
  consensus: DigestItem[];
  divided: DigestItem[];
  groups: DigestGroup[];
};

export type Digest = {
  anchor: string;
  divided_threshold: number;
  topics: TopicDigest[];
};
