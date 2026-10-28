// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useRef, useState } from "react";

import { AnalysisPanel } from "@/components/analysis-panel";
import { VotingPanel } from "@/components/voting-panel";
import type { AnalysisView, NextStatement } from "@/lib/types";

/**
 * 投票と「みんなの意見」をつなぐ。分析を見られるようになったら（`progress.analysis_unlocked`。
 * ルールは backend の1か所）分析を読み込み直し、意見が増えてまたロックされたら読み込み直す。
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
  initialNext,
  initialAnalysis,
}: {
  slug: string;
  topicId: string;
  /** 投票のあいだ上に残す、トピックの説明 */
  intro?: React.ReactNode;
  /** 説明の下を流れて隠れてよいもの（議会での議論・この分野のお金） */
  context?: React.ReactNode;
  /** サーバー側で取ってきた最初の意見と分析（HTML に最初から入れて、表示を速くする） */
  initialNext?: NextStatement | null;
  initialAnalysis?: AnalysisView | null;
}) {
  const [refreshKey, setRefreshKey] = useState(0);
  // 直前の「分析を見られるか」。最初の報告では読み込み直さない（分析側が自分で読み込む）
  const unlocked = useRef<boolean | null>(null);

  const onProgress = useCallback((p: NextStatement["progress"]) => {
    if (unlocked.current !== null && unlocked.current !== p.analysis_unlocked) {
      setRefreshKey((k) => k + 1);
    }
    unlocked.current = p.analysis_unlocked;
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
        <VotingPanel
          slug={slug}
          topicId={topicId}
          onProgress={onProgress}
          initial={initialNext}
        />
      </div>
      <AnalysisPanel
        slug={slug}
        topicId={topicId}
        refreshKey={refreshKey}
        initial={initialAnalysis}
      />
    </>
  );
}
