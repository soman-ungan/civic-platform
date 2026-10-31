// SPDX-License-Identifier: AGPL-3.0-only

// 分析の API の型（backend/src/interface/api/analysis.rs）

import type { ResidencyType } from "./participation";

export type RepfulFor = "agree" | "disagree";

/** グループの代表意見 */
export type RepresentativeStatement = {
  statement_id: string;
  body: string;
  repful_for: RepfulFor;
  n_success: number;
  n_trials: number;
};

export type GroupView = {
  id: number;
  label: string;
  size: number;
  representative: RepresentativeStatement[];
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

export type StatementByGroup = {
  statement_id: string;
  body: string;
  groups: { group_id: number; agree: number; disagree: number; seen: number }[];
};

export type GroupComposition = {
  group_id: number;
  /** 人数。人数の少ない区分は二次秘匿で null */
  by_type: Record<ResidencyType, number | null>;
};

export type AnalysisView =
  | { status: "locked"; remaining: number; total: number }
  | { status: "pending" }
  | { status: "insufficient"; participant_count: number; required: number }
  | {
      status: "ready";
      computed_at: string;
      participant_count: number;
      analyzed_count: number;
      groups: GroupView[];
      consensus: ConsensusView[];
      statements_by_group: StatementByGroup[];
      group_composition: GroupComposition[];
      map: MapView;
      residency: ResidencyRow[];
    };

/** タイムラインにピン留めする合意点 */
export type MunicipalityConsensus = {
  topic_id: string;
  topic_title: string;
  statement_id: string;
  body: string;
  gac: number;
  /** 元のトピックが締め切ったか（締め切ったものはトピックマップに載る） */
  closed: boolean;
};
