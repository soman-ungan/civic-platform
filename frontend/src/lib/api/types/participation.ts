// SPDX-License-Identifier: AGPL-3.0-only

// 投票・投稿・関わり方の API の型（backend/src/interface/api/participation.rs）。
// 参加のルール（backend/src/process/progress.rs）と意見の状態（backend/src/shared/status.rs）も、それを返すここに置く

export type Participation = "none" | "in_progress" | "completed";

/** 投票の進み具合 */
export type Progress = {
  voted: number;
  total: number;
  /** 分析を見られるか（ルールは backend の1か所。画面は写さない） */
  analysis_unlocked: boolean;
};

export type ResidencyType = "resident" | "neighbor" | "commuter" | "related";

export type StatementCard = {
  id: string;
  body: string;
};

export type NextStatement = {
  statement: StatementCard | null;
  progress: Progress;
  needs_relation: boolean;
};

export type VoteValue = 1 | -1 | 0;

/** 意見の状態（backend/src/shared/status.rs） */
export type StatementStatus = "pending" | "approved" | "rejected" | "flagged";

/** 意見を投稿した結果 */
export type PostedStatement = {
  id: string;
  status: StatementStatus;
  /** 非公開になったときの、書き直しの提案 */
  suggestion?: string;
};
