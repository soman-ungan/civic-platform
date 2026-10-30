// SPDX-License-Identifier: AGPL-3.0-only

// トピックの提案の API の型（backend/src/interface/api/proposals.rs）

/** 提案できるか（backend の `progress::propose_permission`。画面はルールを写さない） */
export type ProposePermission =
  | { status: "allowed" }
  | { status: "registration_required" }
  | { status: "relation_required" }
  /** この自治体には、この1週間にもう提案している（次に提案できる日時） */
  | { status: "weekly_limit"; next_at: string };

/** 確認中 / 公開 / 見送り / 公開のあとに取り下げ */
export type ProposalState =
  "reviewing" | "published" | "declined" | "withdrawn";

/** 自分の提案の1件（提案した本人にだけ返る） */
export type Proposal = {
  /** トピックの ID（公開されたら、この ID で開ける） */
  id: string;
  title: string;
  description: string;
  state: ProposalState;
  created_at: string;
};

export type Proposals = {
  permission: ProposePermission;
  items: Proposal[];
};
