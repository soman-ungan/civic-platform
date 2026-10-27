// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { SOURCE_NOTE, fiscalYearLabel, roughYen, shareText } from "@/lib/money";
import type { TopicFinances as TopicFinancesData } from "@/lib/types";

/**
 * この分野のお金。トピックの分野タグに対応する目的別歳出（最新の年度）。
 * 費目は分野より広いので、対応はおおまかだと注記する
 */
export function TopicFinances({
  slug,
  finances,
}: {
  slug: string;
  finances: TopicFinancesData | null;
}) {
  if (!finances) return null;
  return (
    <section
      aria-labelledby="topic-finances"
      className="mt-4 rounded-2xl bg-zinc-50 p-3 dark:bg-zinc-900"
    >
      <h2 id="topic-finances" className="text-sm font-semibold">
        この分野のお金（{fiscalYearLabel(finances.fiscal_year)}）
      </h2>
      <ul className="mt-2 flex flex-col gap-1">
        {finances.items.map((item) => (
          <li
            key={item.key}
            className="flex items-baseline justify-between gap-2 text-sm"
          >
            <span>{item.key}</span>
            <span className="tabular-nums">
              1人あたり{" "}
              {item.per_capita !== null ? roughYen(item.per_capita) : "—"}
              {item.share !== null && (
                <span className="ml-1.5 text-xs text-zinc-500">
                  歳出の{shareText(item.share)}
                </span>
              )}
            </span>
          </li>
        ))}
      </ul>
      <p className="mt-2 text-xs leading-5 text-zinc-500">
        分野と費目の対応はおおまかです（例:
        民生費には福祉と子育ての両方が入ります）。
        {SOURCE_NOTE}。{" "}
        <Link
          href={`/${slug}/finances`}
          className="underline underline-offset-2"
        >
          まちのお金を見る
        </Link>
      </p>
    </section>
  );
}
