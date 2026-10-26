// SPDX-License-Identifier: AGPL-3.0-only

// backend の API レスポンス型（backend/src/routes/*.rs と対応させる）

export type Municipality = {
  id: string;
  slug: string;
  name: string;
  prefecture_name: string;
};

export type District = {
  id: string;
  name: string;
  name_kana: string;
};

export type MunicipalityDetail = Municipality & {
  districts: District[];
};

export type Tag = {
  id: string;
  kind: "field" | "district";
  name: string;
};

export type Topic = {
  id: string;
  title: string;
  description: string;
  source_type: "assembly" | "citizen";
  source_url: string | null;
  created_at: string;
  statement_count: number;
  participant_count: number;
  tags: Tag[];
  /** Cookie の参加者の参加状態 */
  participation: Participation;
  /** Cookie の参加者が投票した、承認済みの意見の数 */
  voted_count: number;
};

export type Participation = "none" | "in_progress" | "completed";

export type TopicList = {
  topics: Topic[];
  tags: Tag[];
};

export type StatementCard = {
  id: string;
  body: string;
};

export type NextStatement = {
  statement: StatementCard | null;
  progress: { voted: number; total: number };
  needs_relation: boolean;
};

export type ResidencyType = "resident" | "neighbor" | "commuter" | "related";

export type VoteValue = 1 | -1 | 0;

export type ApiErrorBody = {
  error: { code: string; message: string };
};

// ===== 分析（backend/src/routes/analysis.rs） =====

export type RepfulFor = "agree" | "disagree";

export type GroupView = {
  id: number;
  label: string;
  size: number;
  representative: {
    statement_id: string;
    body: string;
    repful_for: RepfulFor;
    n_success: number;
    n_trials: number;
  }[];
};

export type ConsensusView = {
  statement_id: string;
  body: string;
  gac: number;
  groups: { group_id: number; label: string; agree: number; seen: number }[];
};

export type MapView = {
  base_clusters: {
    x: number;
    y: number;
    count: number;
    group_id: number | null;
  }[];
  statements: { statement_id: string; x: number; y: number }[];
  you: { x: number; y: number; group_id: number | null } | null;
};

export type ResidencyRow = {
  statement_id: string;
  body: string;
  by_type: Record<ResidencyType, { agree: number; seen: number } | null>;
};

export type AnalysisView =
  | { status: "pending" }
  | { status: "insufficient"; participant_count: number; required: number }
  | {
      status: "ready";
      computed_at: string;
      participant_count: number;
      analyzed_count: number;
      groups: GroupView[];
      consensus: ConsensusView[];
      map: MapView;
      residency: ResidencyRow[];
    };

export type MunicipalityConsensus = {
  topic_id: string;
  topic_title: string;
  statement_id: string;
  body: string;
  gac: number;
};
