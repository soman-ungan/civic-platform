// SPDX-License-Identifier: AGPL-3.0-only

// 議員の記録（backend の interface/api/officials.rs。ADR 0008）

import type { Tag } from "./topics";

export type OfficialRole = "mayor" | "councilor";

export type Official = {
  id: string;
  name: string;
  role: OfficialRole;
  /** 任期の始まり（YYYY-MM-DD。選挙の日） */
  term_start: string;
  /** 任期の終わり。現職は null */
  term_end: string | null;
  /** その人の発言が出典になった、公開済みのトピックの数 */
  topic_count: number;
};

/** 取り込んだ会議録の範囲（最初と最後の会議の日） */
export type Coverage = { from: string; to: string };

export type OfficialList = {
  officials: Official[];
  coverage: Coverage | null;
};

export type OfficialQuote = {
  quote: string;
  document_title: string;
  held_on: string | null;
  url: string | null;
};

export type OfficialTopic = {
  id: string;
  title: string;
  closed: boolean;
  tags: Tag[];
  /** この人の発言の引用（会議録の原文と一致するもの） */
  quotes: OfficialQuote[];
};

export type OfficialDetail = Official & {
  /** 新しい会議の順 */
  topics: OfficialTopic[];
  coverage: Coverage | null;
};
