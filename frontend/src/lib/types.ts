// SPDX-License-Identifier: AGPL-3.0-or-later

// backend の API レスポンス型（backend/src/routes/*.rs と対応させる）

export type Municipality = {
  id: string;
  slug: string;
  name: string;
  prefecture_name: string;
};

export type District = {
  id: string;
  name: string;
  name_kana: string;
};

export type MunicipalityDetail = Municipality & {
  districts: District[];
};

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
};

export type TopicList = {
  topics: Topic[];
  tags: Tag[];
};

export type StatementCard = {
  id: string;
  body: string;
};

export type NextStatement = {
  statement: StatementCard | null;
  progress: { voted: number; total: number };
  needs_relation: boolean;
};

export type ResidencyType = "resident" | "neighbor" | "commuter" | "related";

export type VoteValue = 1 | -1 | 0;

export type ApiErrorBody = {
  error: { code: string; message: string };
};
