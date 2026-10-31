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
  /** 公開した日時（期限の決まりより前に公開したものは null） */
  published_at: string | null;
  /** 新着（公開から3日以内で、まだ投票していない。決まりは backend） */
  is_new: boolean;
  /** 会期（議会のトピックだけ。例「令和8年 第2回定例会（6月）」） */
  session: string | null;
  /** 投票の期限（null は期限なし。ADR 0005） */
  closes_at: string | null;
  /** 締め切ったか（投票と投稿はできない。結果は読める） */
  closed: boolean;
  statement_count: number;
  participant_count: number;
  tags: Tag[];
  /** Cookie の参加者の参加状態 */
  participation: Participation;
  /** Cookie の参加者が投票した、承認済みの意見の数 */
  voted_count: number;
};

export type TopicList = {
  /** 受付中のトピックだけ（締め切ったものは合意点のページの地図から開く） */
  topics: Topic[];
  /** 受付中のトピックに付いているタグ */
  tags: Tag[];
  /** 締め切ったトピックの数 */
  closed_count: number;
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
  /** 市民のまとめから作ったトピックなら、元のトピック */
  parents: TopicLink[];
  /** このトピックから生まれた、公開済みのトピック */
  children: TopicLink[];
};

export type TopicLink = { id: string; title: string };
