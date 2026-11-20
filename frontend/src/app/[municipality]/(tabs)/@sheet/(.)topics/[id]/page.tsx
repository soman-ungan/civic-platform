// SPDX-License-Identifier: AGPL-3.0-only

import { TopicSheet } from "@/components/topics/sheet";

/** タイムラインから開いたトピック詳細（タイムラインの上に重ねるレイヤ。最初から画面の最上部まで伸ばし、ヘッダーも覆う） */
export default function TimelineTopicSheet({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  return <TopicSheet params={params} initialSize="full" />;
}
