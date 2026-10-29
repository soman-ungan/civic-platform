// SPDX-License-Identifier: AGPL-3.0-only

import type { VoteValue } from "@/lib/api/types/participation";

/** 賛成・反対・パスのボタン（投票カードと、似た意見の案内で使う） */
export const VOTE_BUTTONS: {
  value: VoteValue;
  label: string;
  className: string;
}[] = [
  {
    value: 1,
    label: "賛成👍️",
    className: "bg-emerald-600 text-white active:bg-emerald-700",
  },
  {
    value: -1,
    label: "反対👎️",
    className: "bg-rose-600 text-white active:bg-rose-700",
  },
  {
    value: 0,
    label: "パス",
    className:
      "border border-zinc-300 bg-white text-zinc-700 active:bg-zinc-100 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300",
  },
];
