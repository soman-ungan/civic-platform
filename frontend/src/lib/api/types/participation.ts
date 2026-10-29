// SPDX-License-Identifier: AGPL-3.0-only

// 投票・投稿・関わり方の API の型（backend/src/interface/api/participation.rs）。
// 参加のルール（backend/src/process/progress.rs）と意見の状態（backend/src/shared/status.rs）も、それを返すここに置く

export type Participation = "none" | "in_progress" | "completed";

/** 投票の進み具合 */
export type Progress = {
  voted: number;
  total: number;
  /** 投票した最初の意見（議会での主張など）の数。最初の意見は先に出るので、段階を見せるのに使う */
  seeds_voted: number;
  seeds_total: number;
  /** 分析を見られるか（ルールは backend の1か所。画面は写さない） */
  analysis_unlocked: boolean;
  /** 分析を見られるようになるまでの残りの件数（未投票の最初の意見の数） */
  analysis_remaining: number;
};

/** 意見を投稿できるか（backend の `progress::post_permission`。画面はルールを写さない） */
export type PostPermission =
  | { status: "allowed" }
  | { status: "registration_required" }
  | { status: "votes_required"; remaining: number }
  | { status: "limit_reached" };

export type ResidencyType = "resident" | "neighbor" | "commuter" | "related";

export type StatementCard = {
  id: string;
  body: string;
};

export type NextStatement = {
  statement: StatementCard | null;
  progress: Progress;
  needs_relation: boolean;
  posting: PostPermission;
};

export type VoteValue = 1 | -1 | 0;

/** 意見の状態（backend/src/shared/status.rs） */
export type StatementStatus = "pending" | "approved" | "rejected" | "flagged";

/** 非公開の理由の種類（ADR 0007） */
export type Rejection = "harm" | "person" | "official_name" | "not_votable";

/** 意見を投稿した結果 */
export type PostedStatement =
  | {
      id: string;
      status: Exclude<StatementStatus, "rejected">;
    }
  | {
      id: string;
      status: "rejected";
      rejection: Rejection;
      /** 書き直しの提案 */
      suggestion?: string;
    }
  | {
      /** 同じ意見がすでにある（保存しない）。その意見への投票を案内する */
      status: "duplicate";
      similar: SimilarStatement;
    };

/** 似た意見の案内。投票済みなら、その票（票はあとから変えられない） */
export type SimilarStatement = StatementCard & { your_vote?: VoteValue };
