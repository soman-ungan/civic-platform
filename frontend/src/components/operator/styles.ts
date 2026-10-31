// SPDX-License-Identifier: AGPL-3.0-only

// 下書きのカードの、手で直す欄とボタンの見た目（カード・意見の欄・タグの欄で共通）

import { BUTTON_SECONDARY } from "@/components/ui/styles";

/** 文字の入力欄（幅いっぱい） */
export const INPUT =
  "w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm dark:border-zinc-700 dark:bg-zinc-900";

/** 小さい副のボタン（直す・消す・足すなど） */
export const SMALL_BUTTON = `h-8 px-3 text-xs ${BUTTON_SECONDARY}`;
