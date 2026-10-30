// SPDX-License-Identifier: AGPL-3.0-only

import { GROUP_COLORS, percent } from "@/lib/labels";
import type {
  GroupVotes,
  RecommendationItem,
  RecommendationTopic,
} from "@/lib/api/types/recommendations";

/**
 * 提言の項目（合意点・意見が分かれた点）。原文と、作ったときの数字をそのまま出す。
 * グループは元のトピックの分析での意見グループ（属性で分けたものではない。ADR 0002）
 */
export function RecommendationItems({
  items,
  topics,
}: {
  items: RecommendationItem[];
  topics: Map<string, string>;
}) {
  return (
    <ul className="flex flex-col gap-3">
      {items.map((item) => (
        <li
          key={item.statement_id}
          className="rounded-2xl border border-zinc-200 p-4 break-inside-avoid dark:border-zinc-800"
        >
          <p className="leading-7">{item.body}</p>
          <p className="mt-1 text-xs text-zinc-500">
            {topics.get(item.topic_id) ?? "元のトピック"} ・ 賛成{" "}
            {percent(item.agree, item.seen)}% ・ 反対{" "}
            {percent(item.disagree, item.seen)}%（投票 {item.seen} 人）
          </p>
          <ul className="mt-3 flex flex-col gap-1.5">
            {item.groups.map((g, i) => (
              <GroupBar
                key={g.label}
                group={g}
                color={GROUP_COLORS[i % GROUP_COLORS.length]}
              />
            ))}
          </ul>
        </li>
      ))}
    </ul>
  );
}

function GroupBar({ group, color }: { group: GroupVotes; color: string }) {
  const agree = percent(group.agree, group.seen);
  const disagree = percent(group.disagree, group.seen);
  return (
    <li className="flex items-center gap-2 text-xs">
      <span className="w-14 shrink-0 font-semibold" style={{ color }}>
        グループ{group.label}
      </span>
      <span
        aria-hidden
        className="flex h-2 min-w-0 flex-1 overflow-hidden rounded-full bg-zinc-100 dark:bg-zinc-800"
      >
        <span className="bg-sky-600" style={{ width: `${agree}%` }} />
        <span className="bg-rose-500" style={{ width: `${disagree}%` }} />
      </span>
      <span className="w-32 shrink-0 text-right tabular-nums text-zinc-600 dark:text-zinc-400">
        賛成{agree}% 反対{disagree}%（{group.seen}人）
      </span>
    </li>
  );
}

/** 元のトピックと、参加した人数・関わり方ごとの人数（少人数の区分は隠す） */
export function RecommendationTopics({
  slug,
  topics,
  labels,
}: {
  slug: string;
  topics: RecommendationTopic[];
  labels: [key: string, label: string][];
}) {
  return (
    <ul className="flex flex-col gap-3">
      {topics.map((t) => (
        <li key={t.id} className="text-sm">
          <a
            href={`/${slug}/topics/${t.id}`}
            className="font-medium underline underline-offset-2"
          >
            {t.title}
          </a>
          <p className="mt-0.5 text-xs leading-5 text-zinc-500">
            分析の対象 {t.participants} 人（
            {labels
              .map(([key, label]) => {
                const n = t.residency[key as keyof typeof t.residency];
                return `${label} ${n == null ? "—" : `${n}人`}`;
              })
              .join("・")}
            ）
          </p>
        </li>
      ))}
    </ul>
  );
}
