// SPDX-License-Identifier: AGPL-3.0-only

// 予算の主な事業の表示に使うもの

import type { BudgetProject } from "@/lib/types";

/** 新規・拡充の印の文字 */
export const FLAG_LABEL: Record<NonNullable<BudgetProject["flag"]>, string> = {
  new: "新規",
  expanded: "拡充",
};
