// SPDX-License-Identifier: AGPL-3.0-only

import { TopicSheet } from "@/components/topics/sheet";
import { MUNICIPALITY_NAME_ID } from "@/lib/tabs";

/** タイムラインから開いたトピック詳細（タイムラインの上に重ねるレイヤ。最初から自治体名の下まで伸ばし、タブは隠す） */
export default function TimelineTopicSheet({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  return (
    <TopicSheet
      params={params}
      initialSize="full"
      topAnchorId={MUNICIPALITY_NAME_ID}
    />
  );
}
