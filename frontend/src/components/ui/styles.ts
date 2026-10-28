// SPDX-License-Identifier: AGPL-3.0-only

// よく使う見た目のクラス（大きさや余白は使う側で足す）

/** 主のボタン（黒地） */
export const BUTTON_PRIMARY =
  "rounded-full bg-zinc-900 font-medium text-white disabled:opacity-40 dark:bg-zinc-100 dark:text-zinc-900";

/** 副のボタン（枠だけ） */
export const BUTTON_SECONDARY =
  "rounded-full border border-zinc-300 disabled:opacity-50 dark:border-zinc-700";

/** 黄色の印（予算・新規/拡充・投票の途中など。形は使う側で足す） */
export const BADGE_AMBER =
  "bg-amber-100 text-amber-900 dark:bg-amber-900/60 dark:text-amber-100";

/** エラーの文 */
export const ERROR_TEXT = "text-sm text-rose-600";
