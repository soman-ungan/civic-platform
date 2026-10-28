// SPDX-License-Identifier: AGPL-3.0-only

// まちのお金の API の型（backend/src/interface/api/finances.rs）

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
