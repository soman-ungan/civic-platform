// SPDX-License-Identifier: AGPL-3.0-only

import type { ReadyAnalysis } from "@/lib/api/types/analysis";

/** 使い方のページの架空の例題（example.ts。scripts/generate_tutorial_example.py で生成） */
export type GuideExample = {
  title: string;
  description: string;
  statements: { id: string; body: string }[];
  analysis: ReadyAnalysis;
};
