// SPDX-License-Identifier: AGPL-3.0-only

// まちのお金で、選んだ年度と「決算／予算」の切り替えから、画面に出す数字を選ぶ

import type { Unit } from "./money";
import type {
  FinanceAmount,
  FinanceKind,
  Finances,
} from "@/lib/api/types/finances";

export const KIND_LABEL: Record<FinanceKind, string> = {
  settlement: "決算",
  budget: "予算",
};

/** 選んだ年度・切り替えで出す数字（金額は千円、1人あたりは円） */
export type FinanceView = {
  kind: FinanceKind;
  /** その年度にある種類（両方あれば切り替えを出す） */
  kinds: FinanceKind[];
  total: number | null;
  population: number | null;
  /**
   * 予算のときに、別の年度の人口で割っているならその年度（予算の年度の人口はまだ無いので、それ以前で最新のもの）。
   * 決算が出ていて同じ年度の人口があるときは null（「（◯年度）」と断らなくてよい）
   */
  populationYear: number | null;
  perCapita: number | null;
  purpose: FinanceAmount[];
  nature: FinanceAmount[];
  revenue: FinanceAmount[];
  revenueTotal: number | null;
};

/**
 * `requested`（URL の ?view=）があればそれを、無ければ決算を出す。選んだ方が無い年度は、ある方を出す
 */
export function financeView(
  f: Finances,
  requested: string | undefined,
): FinanceView {
  const kinds: FinanceKind[] = [];
  if (f.settlement) kinds.push("settlement");
  if (f.budget) kinds.push("budget");
  const kind: FinanceKind =
    requested === "budget" && f.budget
      ? "budget"
      : f.settlement
        ? "settlement"
        : "budget";
  const s = kind === "settlement" ? f.settlement : null;
  const b = kind === "budget" ? f.budget : null;
  const total = s ? s.expenditure_total : (b?.total ?? null);
  const population = s ? s.population : (b?.population ?? null);
  return {
    kind,
    kinds,
    total,
    population,
    populationYear:
      b?.population_year != null && b.population_year !== f.fiscal_year
        ? b.population_year
        : null,
    perCapita:
      total !== null && population ? (total * 1000) / population : null,
    purpose: (s ?? b)?.expenditure_purpose ?? [],
    nature: (s ?? b)?.expenditure_nature ?? [],
    revenue: (s ?? b)?.revenue ?? [],
    revenueTotal: s ? s.revenue_total : (b?.total ?? null),
  };
}

/**
 * まちのお金の URL の ?以降。切り替え（決算／予算、1人あたり／総額）は、既定でないときだけ付ける。
 * 年度のチップ・推移の棒・切り替えのどれから変えても、同じ形にする
 */
export function financesQuery(
  year: number,
  kind: FinanceKind,
  unit: Unit,
): string {
  return `?year=${year}${kind === "budget" ? "&view=budget" : ""}${
    unit === "per_capita" ? "&unit=per_capita" : ""
  }`;
}

export function financesHref(
  slug: string,
  year: number,
  kind: FinanceKind,
  unit: Unit,
): string {
  return `/${slug}/finances${financesQuery(year, kind, unit)}`;
}

/** URL の ?unit= から単位を読む（既定は総額。2026-11-08 に1人あたりから変えた） */
export function parseUnit(value: unknown): Unit {
  return value === "per_capita" ? "per_capita" : "total";
}
