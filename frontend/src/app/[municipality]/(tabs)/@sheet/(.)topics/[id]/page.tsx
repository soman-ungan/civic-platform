// SPDX-License-Identifier: AGPL-3.0-only

import { TopicSheet } from "@/components/topics/sheet";
import { TABS_HEADER_ID } from "@/lib/tabs";

/** タイムラインから開いたトピック詳細（タイムラインの上に重ねるレイヤ。最初から、自治体名とタブのヘッダーの下まで伸ばす） */
export default function TimelineTopicSheet({
  params,
}: PageProps<"/[municipality]/topics/[id]">) {
  return (
    <TopicSheet
      params={params}
      initialSize="full"
      topAnchorId={TABS_HEADER_ID}
    />
  );
}
