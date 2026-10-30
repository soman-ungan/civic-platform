// SPDX-License-Identifier: AGPL-3.0-only

import { GROUP_COLORS, percent } from "@/lib/labels";
import type {
  DigestGroup,
  DigestItem,
  GroupVotes,
} from "@/lib/api/types/digest";

/** 意見を「最初の意見に使う」の切り替え（まとめから提案するとき） */
export type UseToggle = {
  selected: Set<string>;
  onToggle: (statementId: string) => void;
};

/**
 * 合意点・意見が分かれた点。原文と、分析に含まれた人の票をそのまま出す。
 * グループはそのトピックの投票の傾向で分けた意見のグループ（属性で分けたものではない。ADR 0002）
 */
export function DigestItems({
  items,
  use,
}: {
  items: DigestItem[];
  use?: UseToggle;
}) {
  if (items.length === 0) {
    return <p className="text-sm text-zinc-500">ありません</p>;
  }
  return (
    <ul className="flex flex-col gap-3">
      {items.map((item) => (
        <li
          key={item.statement_id}
          className="rounded-2xl border border-zinc-200 p-4 break-inside-avoid dark:border-zinc-800"
        >
          <div className="flex items-start gap-3">
            <p className="min-w-0 flex-1 leading-7">{item.body}</p>
            {use && <UseButton statementId={item.statement_id} use={use} />}
          </div>
          <p className="mt-1 text-xs text-zinc-500">
            賛成 {percent(item.agree, item.seen)}% ・ 反対{" "}
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
      <span className="w-32 shrink-0 text-right text-zinc-600 tabular-nums dark:text-zinc-400">
        賛成{agree}% 反対{disagree}%（{group.seen}人）
      </span>
    </li>
  );
}

/** グループごとの代表的な意見（Polis の repness。AI の要約ではなく原文） */
export function DigestGroups({
  groups,
  use,
}: {
  groups: DigestGroup[];
  use?: UseToggle;
}) {
  return (
    <ul className="flex flex-col gap-3">
      {groups.map((g, i) => (
        <li
          key={g.label}
          className="rounded-2xl border border-zinc-200 p-4 break-inside-avoid dark:border-zinc-800"
        >
          <p
            className="text-sm font-semibold"
            style={{ color: GROUP_COLORS[i % GROUP_COLORS.length] }}
          >
            グループ{g.label}（{g.members}人）
          </p>
          <ul className="mt-2 flex flex-col gap-2">
            {g.representative.map((r) => (
              <li key={r.statement_id} className="flex items-start gap-3">
                <p className="min-w-0 flex-1 text-sm leading-6">
                  <span className="mr-1 text-xs text-zinc-500">
                    {r.repful_for === "agree" ? "多くが賛成" : "多くが反対"}
                  </span>
                  {r.body}
                </p>
                {use && <UseButton statementId={r.statement_id} use={use} />}
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  );
}

function UseButton({
  statementId,
  use,
}: {
  statementId: string;
  use: UseToggle;
}) {
  const on = use.selected.has(statementId);
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={() => use.onToggle(statementId)}
      className={`h-8 shrink-0 rounded-full border px-3 text-xs print:hidden ${
        on
          ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
          : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300"
      }`}
    >
      {on ? "使う ✓" : "使う"}
    </button>
  );
}
