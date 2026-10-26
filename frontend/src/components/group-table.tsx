// SPDX-License-Identifier: AGPL-3.0-only

import { GROUP_COLORS, percent } from "@/lib/labels";
import type { GroupView, StatementByGroup } from "@/lib/types";

/**
 * グループごとの特徴: 行 = 意見、列 = グループ。各グループの賛成・反対の割合を一覧する。
 * スマホでは横にスクロールでき、意見の列は左端に固定する。自分のグループの列は強調する。
 */
export function GroupTable({
  groups,
  rows,
  myGroupId,
}: {
  groups: GroupView[];
  rows: StatementByGroup[];
  myGroupId: number | null;
}) {
  // Polis が代表意見に選んだ（そのグループらしさが強い）意見
  const representative = new Map(
    groups.map((g) => [
      g.id,
      new Set(g.representative.map((r) => r.statement_id)),
    ]),
  );

  return (
    <div>
      <h3 className="mb-1 font-semibold">グループごとの特徴</h3>
      <p className="mb-2 text-xs leading-5 text-zinc-500">
        各グループで賛成した人の割合（下段は反対）。★
        はそのグループの特徴をよく表す意見です。表は横にスクロールできます。
      </p>
      <div className="overflow-x-auto rounded-xl border border-zinc-200 dark:border-zinc-800">
        <table className="w-max min-w-full border-separate border-spacing-0 text-left text-xs">
          <thead>
            <tr>
              <th
                scope="col"
                className="sticky left-0 z-10 w-40 min-w-40 border-b border-zinc-200 bg-background px-3 py-2 font-normal text-zinc-500 dark:border-zinc-800"
              >
                意見
              </th>
              {groups.map((g, i) => {
                const mine = g.id === myGroupId;
                return (
                  <th
                    key={g.id}
                    scope="col"
                    className={`w-24 min-w-24 border-b border-zinc-200 px-2 py-2 text-center align-bottom dark:border-zinc-800 ${
                      mine ? "bg-sky-50 dark:bg-sky-950/50" : ""
                    }`}
                  >
                    {mine && (
                      <span className="mb-1 inline-block rounded-full bg-sky-600 px-2 py-0.5 text-[10px] font-bold text-white">
                        あなた
                      </span>
                    )}
                    <span className="flex items-center justify-center gap-1 font-semibold">
                      <span
                        aria-hidden
                        className="inline-block size-2.5 rounded-full"
                        style={{
                          backgroundColor:
                            GROUP_COLORS[i % GROUP_COLORS.length],
                        }}
                      />
                      {g.label}
                    </span>
                    <span className="block font-normal text-zinc-500">
                      {g.size}人
                    </span>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.statement_id}>
                <th
                  scope="row"
                  className="sticky left-0 z-10 w-40 min-w-40 border-b border-zinc-200 bg-background px-3 py-2 align-top font-normal leading-5 dark:border-zinc-800"
                >
                  {row.body}
                </th>
                {row.groups.map((cell) => {
                  const mine = cell.group_id === myGroupId;
                  const rep = representative
                    .get(cell.group_id)
                    ?.has(row.statement_id);
                  return (
                    <td
                      key={cell.group_id}
                      className={`border-b border-zinc-200 px-2 py-2 text-center align-top dark:border-zinc-800 ${
                        mine ? "bg-sky-50 dark:bg-sky-950/50" : ""
                      }`}
                    >
                      <VoteCell
                        agree={cell.agree}
                        disagree={cell.disagree}
                        seen={cell.seen}
                        representative={rep ?? false}
                      />
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function VoteCell({
  agree,
  disagree,
  seen,
  representative,
}: {
  agree: number;
  disagree: number;
  seen: number;
  representative: boolean;
}) {
  if (seen === 0) {
    return <span className="text-zinc-400">—</span>;
  }
  const a = percent(agree, seen);
  const d = percent(disagree, seen);
  return (
    <div className="flex flex-col items-center gap-0.5">
      <span className="text-sm font-semibold text-emerald-700 dark:text-emerald-400">
        {representative && (
          <span aria-label="代表意見" className="mr-0.5 text-amber-500">
            ★
          </span>
        )}
        {a}%
      </span>
      <span className="text-[11px] text-rose-700 dark:text-rose-400">
        反対 {d}%
      </span>
      {/* 賛成・反対・パスの内訳 */}
      <span
        aria-hidden
        className="mt-0.5 flex h-1.5 w-14 overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-700"
      >
        <span className="bg-emerald-500" style={{ width: `${a}%` }} />
        <span className="bg-rose-500" style={{ width: `${d}%` }} />
      </span>
      <span className="text-[10px] text-zinc-500">{seen}人</span>
    </div>
  );
}
