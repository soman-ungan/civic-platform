// SPDX-License-Identifier: AGPL-3.0-only

// トピックの API の型（backend/src/interface/api/topics.rs）

import type { TopicFinanceData } from "./finances";
import type { Participation } from "./participation";

export type Tag = {
  id: string;
  kind: "field" | "district";
  name: string;
};

export type Topic = {
  id: string;
  title: string;
  description: string;
  source_type: "assembly" | "citizen";
  source_url: string | null;
  created_at: string;
  statement_count: number;
  participant_count: number;
  tags: Tag[];
  /** Cookie の参加者の参加状態 */
  participation: Participation;
  /** Cookie の参加者が投票した、承認済みの意見の数 */
  voted_count: number;
};

export type TopicList = {
  topics: Topic[];
  tags: Tag[];
};

/** 議会のトピックの出典（会議録の発言からの引用） */
export type TopicSource = {
  /** 公人は氏名つき（「〇〇議員」）、職員は職名だけ */
  speaker: string;
  quote: string;
  document_title: string;
  held_on: string | null;
  url: string | null;
};

export type TopicDetail = Topic & {
  sources: TopicSource[];
  /** トピックの分野に関係する市のお金。対応する費目が無ければ null */
  finances: TopicFinanceData | null;
};
