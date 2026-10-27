// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useRef, useState } from "react";

import { AnalysisPanel } from "@/components/analysis-panel";
import { VotingPanel } from "@/components/voting-panel";
import type { NextStatement } from "@/lib/types";

/**
 * 投票と「みんなの意見」をつなぐ。すべての意見に投票し終えたら分析を読み込み直し、
 * 意見が増えて未投票ができたら（分析はロックされるので）また読み込み直す。
 *
 * 画面は2段でスクロールする。投票の部分では、トピックの説明（`intro`）を上に貼り付け、
 * 議会での議論やお金（`context`）はその下を流れて隠れる。「みんなの意見」まで進むと、
 * 投票の部分が終わるので、説明も一緒に流れて隠れる（sticky は親の範囲の中でだけ貼り付く）
 */
export function TopicInteraction({
  slug,
  topicId,
  intro,
  context,
}: {
  slug: string;
  topicId: string;
  /** 投票のあいだ上に残す、トピックの説明 */
  intro?: React.ReactNode;
  /** 説明の下を流れて隠れてよいもの（議会での議論・この分野のお金） */
  context?: React.ReactNode;
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
      <div>
        {intro && (
          <div className="sticky top-0 z-10 -mx-4 border-b border-zinc-200 bg-background/95 px-4 pt-3 pb-3 backdrop-blur dark:border-zinc-800">
            {intro}
          </div>
        )}
        {context}
        <VotingPanel slug={slug} topicId={topicId} onProgress={onProgress} />
      </div>
      <AnalysisPanel slug={slug} topicId={topicId} refreshKey={refreshKey} />
    </>
  );
}
