// SPDX-License-Identifier: AGPL-3.0-only

// まちのお金で、選んだ年度と「決算／予算」の切り替えから、画面に出す数字を選ぶ

import type { Unit } from "@/lib/money";
import type { FinanceAmount, FinanceKind, Finances } from "@/lib/types";

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
  /** 予算のときの人口の年度（予算の年度の人口はまだ無いので、それ以前で最新のもの） */
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
    populationYear: b?.population_year ?? null,
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
    unit === "total" ? "&unit=total" : ""
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

/** URL の ?unit= から単位を読む（既定は1人あたり） */
export function parseUnit(value: unknown): Unit {
  return value === "total" ? "total" : "per_capita";
}
