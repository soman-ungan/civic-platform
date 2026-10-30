// SPDX-License-Identifier: AGPL-3.0-only

import { TopicSheet } from "@/components/topics/sheet";

/** 合意点のページから開いたトピック詳細（ページの上に重ねるレイヤ） */
export default function ConsensusTopicSheet({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  return <TopicSheet params={params} />;
}
