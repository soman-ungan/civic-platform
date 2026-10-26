// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useRef, useState } from "react";

import { AnalysisPanel } from "@/components/analysis-panel";
import { VotingPanel } from "@/components/voting-panel";
import type { NextStatement } from "@/lib/types";

/**
 * 投票と「みんなの意見」をつなぐ。すべての意見に投票し終えたら分析を読み込み直し、
 * 意見が増えて未投票ができたら（分析はロックされるので）また読み込み直す。
 */
export function TopicInteraction({
  slug,
  topicId,
}: {
  slug: string;
  topicId: string;
}) {
  const [refreshKey, setRefreshKey] = useState(0);
  // 直前の「投票し終えているか」。最初の報告では読み込み直さない（分析側が自分で読み込む）
  const completed = useRef<boolean | null>(null);

  const onProgress = useCallback((p: NextStatement["progress"]) => {
    const done = p.total > 0 && p.voted >= p.total;
    if (completed.current !== null && completed.current !== done) {
      setRefreshKey((k) => k + 1);
    }
    completed.current = done;
  }, []);

  return (
    <>
      <VotingPanel slug={slug} topicId={topicId} onProgress={onProgress} />
      <AnalysisPanel slug={slug} topicId={topicId} refreshKey={refreshKey} />
    </>
  );
}
