// SPDX-License-Identifier: AGPL-3.0-only

// 運営者の API の型（backend/src/interface/api/operator.rs）。投稿者が誰かは運営者にも返らない

/** トピックの出典（会議録の発言からの引用） */
export type OperatorSource = {
  document_title: string;
  held_on: string | null;
  url: string | null;
  utterance_no: number;
  speaker: string;
  quote: string;
};

export type OperatorStatement = {
  id: string;
  body: string;
  status: "approved" | "pending" | "flagged";
  /** 根拠の発言（「発言12 〇〇議員」）。会議録から作った意見だけ */
  basis: string[];
};

export type OperatorTopic = {
  id: string;
  status: "draft" | "published" | "archived";
  /** 会議録から / 市民の提案（提案した人は返らない） */
  source_type: "assembly" | "citizen";
  title: string;
  description: string;
  created_at: string;
  published_at: string | null;
  /** 公開待ちに入れた日時（下書きだけ。null は入っていない） */
  queued_at: string | null;
  closes_at: string | null;
  closed: boolean;
  sources: OperatorSource[];
  statements: OperatorStatement[];
  tags: { id: string; name: string; assigned_by: "ai" | "admin" }[];
};

/** 付けられるタグ（分野と、その自治体の地区） */
export type TagChoice = {
  id: string;
  kind: "field" | "district";
  name: string;
};

/** 確認待ち（flagged）・判定待ち（pending）の意見 */
export type QueueStatement = {
  id: string;
  topic_id: string;
  topic_title: string;
  topic_status: OperatorTopic["status"];
  status: "flagged" | "pending";
  body: string;
  /** AI・運営者の判定の理由 */
  moderation_note: string | null;
  created_at: string;
  /** seed: 最初の意見 / citizen: 市民の投稿 / added: あとから足した議会の主張・運営者の意見 */
  kind: "seed" | "citizen" | "added";
  basis: string[];
  document: {
    title: string;
    held_on: string | null;
    url: string | null;
  } | null;
};

/** 公開待ちの1件（公開する順） */
export type QueuedTopic = {
  id: string;
  title: string;
  source_type: "assembly" | "citizen";
  queued_at: string;
  /** 公開の予定（今の時刻の設定のままの見込み）。時刻が1つも無ければ null */
  planned_at: string | null;
};

/** 自動公開の時刻（1つで1日1件） */
export type PublishSlot = {
  id: string;
  /** 日本時間の HH:MM */
  time_of_day: string;
};

/** 自動公開の全体（docs/plans/2026-10-30-timeline-flow.md） */
export type Publishing = {
  queue: QueuedTopic[];
  slots: PublishSlot[];
};
