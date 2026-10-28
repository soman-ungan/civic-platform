// SPDX-License-Identifier: AGPL-3.0-only

// backend の API レスポンス型（backend/src/ のファイルごとに並べ、名前も対応させる）

/** エラー（backend/src/error.rs） */
export type ApiErrorBody = {
  error: { code: string; message: string };
};

// ===== 自治体（backend/src/routes/municipalities.rs） =====

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

// ===== トピック（backend/src/routes/topics.rs） =====

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

export type TopicList = {
  topics: Topic[];
  tags: Tag[];
};

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
  /** トピックの分野に関係する市のお金。対応する費目が無ければ null */
  finances: TopicFinanceData | null;
};

// ===== 参加のルール（backend/src/progress.rs） =====

export type Participation = "none" | "in_progress" | "completed";

/** 投票の進み具合 */
export type Progress = {
  voted: number;
  total: number;
  /** 分析を見られるか（ルールは backend の1か所。画面は写さない） */
  analysis_unlocked: boolean;
};

// ===== 投票・投稿・関わり方（backend/src/routes/participation.rs） =====

export type ResidencyType = "resident" | "neighbor" | "commuter" | "related";

export type StatementCard = {
  id: string;
  body: string;
};

export type NextStatement = {
  statement: StatementCard | null;
  progress: Progress;
  needs_relation: boolean;
};

export type VoteValue = 1 | -1 | 0;

/** 意見の状態（backend/src/status.rs） */
export type StatementStatus = "pending" | "approved" | "rejected" | "flagged";

/** 意見を投稿した結果 */
/** ログインの状態（backend/src/interface/api/auth.rs。利用者の ID は返さない） */
export type Me = {
  /** パスキーを登録している（意見を投稿できる） */
  registered: boolean;
  operator: boolean;
};

export type PostedStatement = {
  id: string;
  status: StatementStatus;
  /** 非公開になったときの、書き直しの提案 */
  suggestion?: string;
};

// ===== 分析（backend/src/routes/analysis.rs） =====

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

/** 決算（総務省の決算カード）か、当初予算か */
export type FinanceKind = "settlement" | "budget";

export type FinanceYear = {
  fiscal_year: number;
  /** 決算（決算カード）があるか */
  settlement: boolean;
  /** 公開済みの予算があるか */
  budget: boolean;
};

/** ある年度の決算（総務省の決算カード） */
export type Settlement = {
  population: number | null;
  revenue_total: number | null;
  expenditure_total: number | null;
  revenue: FinanceAmount[];
  expenditure_purpose: FinanceAmount[];
  expenditure_nature: FinanceAmount[];
  indicators: FinanceIndicator[];
  source: FinanceSource;
};

export type ProjectTag = {
  id: string;
  name: string;
};

/** 主な事業。説明の文章は持たない（原本の PDF のページへリンクする） */
export type BudgetProject = {
  name: string;
  /** 千円 */
  amount: number;
  per_capita: number | null;
  flag: "new" | "expanded" | null;
  /** その自治体での政策・分野の名前（無い自治体もある） */
  plan_label: string | null;
  page: number;
  tags: ProjectTag[];
};

export type BudgetSource = {
  title: string;
  /** 原本の PDF */
  url: string | null;
};

/** ある年度の当初予算（自治体の「当初予算の概要」） */
export type Budget = {
  /** 一般会計の歳出の合計（千円） */
  total: number;
  /** 1人あたりの換算に使った人口と、その年度（予算の年度の人口はまだ無い） */
  population: number | null;
  population_year: number | null;
  revenue: FinanceAmount[];
  expenditure_purpose: FinanceAmount[];
  expenditure_nature: FinanceAmount[];
  projects: BudgetProject[];
  source: BudgetSource;
};

/** 推移の1年度分 */
export type TrendYear = {
  fiscal_year: number;
  /** budget は当初予算（補正を含まないので、決算より小さく出る） */
  kind: FinanceKind;
  /** 予算の年度は、それ以前で最新の人口 */
  population: number | null;
  /** 目的別歳出（千円） */
  expenditure_purpose: Record<string, number | null>;
};

export type Finances = {
  years: FinanceYear[];
  fiscal_year: number;
  settlement: Settlement | null;
  budget: Budget | null;
  /** 推移（目的別歳出。古い順）。決算の年度と、決算がまだ無く予算だけがある年度 */
  trend: TrendYear[];
};

/** トピックの分野の、今年度の主な事業（5件まで） */
export type TopicBudget = {
  fiscal_year: number;
  projects: BudgetProject[];
  project_count: number;
  source: BudgetSource;
};

/** トピックの詳細の `finances`（分野タグに対応する目的別歳出と、主な事業） */
export type TopicFinanceData = {
  fiscal_year: number;
  population: number | null;
  items: FinanceAmount[];
  source: FinanceSource;
  budget: TopicBudget | null;
};
