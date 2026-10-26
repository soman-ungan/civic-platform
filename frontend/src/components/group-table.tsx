// SPDX-License-Identifier: AGPL-3.0-only

import { GROUP_COLORS, percent } from "@/lib/labels";
import type { GroupView, RepfulFor, StatementByGroup } from "@/lib/types";

/** 自分のグループの列を囲む枠線の色（凡例の色見本用） */
const MY_BORDER = "border-sky-500 dark:border-sky-400";
/** 自分の列: 左右は青い枠、行の区切りは他の列と同じ灰色 */
const MY_SIDES = "border-x-2 border-x-sky-500 dark:border-x-sky-400";
const ROW_LINE = "border-b border-b-zinc-200 dark:border-b-zinc-800";
/** 自分の列の外側の上端・下端 */
const MY_TOP = "border-t-2 border-t-sky-500 dark:border-t-sky-400";
const MY_BOTTOM = "border-b-2 border-b-sky-500 dark:border-b-sky-400";

/** 代表意見のセルの塗り（そのグループの特徴が賛成向きか反対向きか） */
const HIGHLIGHT: Record<RepfulFor, string> = {
  agree: "bg-emerald-100 dark:bg-emerald-900/40",
  disagree: "bg-rose-100 dark:bg-rose-900/40",
};

/**
 * グループごとの特徴: 行 = 意見、列 = グループ。各グループの賛成・反対の割合を一覧する。
 * Polis が代表意見に選んだセルは、賛成向きなら緑、反対向きなら赤で塗る。
 * スマホでは横にスクロールでき、意見の列は左端に固定する。自分のグループの列は青い枠で囲む。
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
  // グループ → 意見 → 代表意見としての向き
  const representative = new Map(
    groups.map((g) => [
      g.id,
      new Map(g.representative.map((r) => [r.statement_id, r.repful_for])),
    ]),
  );

  return (
    <div>
      <h3 className="mb-1 font-semibold">グループごとの特徴</h3>
      <div className="mb-2 flex flex-col gap-1 text-xs leading-5 text-zinc-500">
        <p>各グループで 👍️ 賛成 / 👎️ 反対した人の割合です。</p>
        <p className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <span className="inline-flex items-center gap-1">
            <span
              aria-hidden
              className={`inline-block size-3 rounded-sm ${HIGHLIGHT.agree}`}
            />
            そのグループの特徴（賛成が特に多い）
          </span>
          <span className="inline-flex items-center gap-1">
            <span
              aria-hidden
              className={`inline-block size-3 rounded-sm ${HIGHLIGHT.disagree}`}
            />
            そのグループの特徴（反対が特に多い）
          </span>
          {myGroupId !== null && (
            <span className="inline-flex items-center gap-1">
              <span
                aria-hidden
                className={`inline-block size-3 rounded-sm border-2 ${MY_BORDER}`}
              />
              あなたのグループ
            </span>
          )}
        </p>
        <p>表は横にスクロールできます。</p>
      </div>
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
                    className={`w-24 min-w-24 px-2 py-2 text-center align-bottom ${
                      mine
                        ? // 青い枠は列の外側だけ。見出しの下は他の列と同じ灰色の線
                          `${MY_SIDES} ${MY_TOP} ${ROW_LINE}`
                        : "border-b border-zinc-200 dark:border-zinc-800"
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
            {rows.map((row, rowIndex) => {
              const last = rowIndex === rows.length - 1;
              return (
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
                      ?.get(row.statement_id);
                    // 自分の列: 青い枠は外側だけ、行の区切りは灰色（最後の行の下端だけ青）
                    const border = mine
                      ? `${MY_SIDES} ${last ? MY_BOTTOM : ROW_LINE}`
                      : "border-b border-zinc-200 dark:border-zinc-800";
                    return (
                      <td
                        key={cell.group_id}
                        className={`px-2 py-2 text-center align-top ${border} ${
                          rep ? HIGHLIGHT[rep] : ""
                        }`}
                      >
                        <VoteCell
                          agree={cell.agree}
                          disagree={cell.disagree}
                          seen={cell.seen}
                          representative={rep ?? null}
                        />
                      </td>
                    );
                  })}
                </tr>
              );
            })}
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
  /** 代表意見なら、その向き */
  representative: RepfulFor | null;
}) {
  if (seen === 0) {
    return <span className="text-zinc-400">—</span>;
  }
  const a = percent(agree, seen);
  const d = percent(disagree, seen);
  return (
    <div className="flex flex-col items-center gap-0.5">
      {representative && (
        <span className="sr-only">
          このグループの特徴（
          {representative === "agree" ? "賛成" : "反対"}が多い）
        </span>
      )}
      <span
        className={`text-sm text-emerald-700 dark:text-emerald-400 ${
          representative === "agree" ? "font-bold" : ""
        }`}
      >
        <span aria-hidden>👍️ </span>
        <span className="sr-only">賛成 </span>
        {a}%
      </span>
      <span
        className={`text-sm text-rose-700 dark:text-rose-400 ${
          representative === "disagree" ? "font-bold" : ""
        }`}
      >
        <span aria-hidden>👎️ </span>
        <span className="sr-only">反対 </span>
        {d}%
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
