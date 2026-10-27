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

/** 議会のトピックの出典（会議録の発言からの引用） */
export type TopicSource = {
  /** 公人は氏名つき（「〇〇議員」）、職員は職名だけ */
  speaker: string;
  quote: string;
  document_title: string;
  held_on: string | null;
  url: string | null;
};

export type TopicDetail = Topic & {
  sources: TopicSource[];
  /** トピックの分野に関係する市のお金（決算カード）。対応する費目が無ければ null */
  finances: TopicFinances | null;
};

// ===== まちのお金（backend/src/routes/finances.rs） =====

export type FinanceAmount = {
  key: string;
  /** 千円。原表が「-」なら null */
  value: number | null;
  /** 市民1人あたり（円） */
  per_capita: number | null;
  /** 区分の合計に占める割合（0〜1） */
  share: number | null;
};

export type FinanceIndicator = {
  key: string;
  value: number | null;
  unit: "thousand_yen" | "ratio";
};

export type FinanceSource = {
  title: string;
  /** 決算カードの年度のページ */
  page_url: string | null;
  excel_url: string | null;
};

export type Finances = {
  years: number[];
  fiscal_year: number;
  population: number | null;
  revenue_total: number | null;
  expenditure_total: number | null;
  revenue: FinanceAmount[];
  expenditure_purpose: FinanceAmount[];
  expenditure_nature: FinanceAmount[];
  indicators: FinanceIndicator[];
  trend: {
    fiscal_year: number;
    population: number | null;
    /** 目的別歳出（千円） */
    expenditure_purpose: Record<string, number | null>;
  }[];
  source: FinanceSource;
};

export type TopicFinances = {
  fiscal_year: number;
  population: number | null;
  items: FinanceAmount[];
  source: FinanceSource;
};

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

export type MunicipalityConsensus = {
  topic_id: string;
  topic_title: string;
  statement_id: string;
  body: string;
  gac: number;
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
