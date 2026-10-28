// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { BADGE_AMBER } from "@/components/ui/styles";
import { FLAG_LABEL } from "@/lib/finances/budget-fields";
import {
  SOURCE_NOTE,
  fiscalYearLabel,
  roughYen,
  shareText,
  thousandYen,
} from "@/lib/finances/money";
import type { TopicFinanceData } from "@/lib/types";

/**
 * この分野のお金。トピックの分野タグに対応する目的別歳出（決算の最新の年度）と、
 * 同じ分野の今年度の主な事業（予算）。費目は分野より広いので、対応はおおまかだと注記する。
 * 事業の説明の文章は載せず、原本のページへリンクする（市の著作物のため）
 */
export function TopicFinances({
  slug,
  finances,
}: {
  slug: string;
  finances: TopicFinanceData | null;
}) {
  if (!finances) return null;
  const budget = finances.budget;
  return (
    <section
      aria-labelledby="topic-finances"
      className="mt-4 rounded-2xl bg-zinc-50 p-3 dark:bg-zinc-900"
    >
      {finances.items.length > 0 && (
        <>
          <h2 id="topic-finances" className="text-sm font-semibold">
            この分野のお金（{fiscalYearLabel(finances.fiscal_year)}・決算）
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
            民生費には福祉と子育ての両方が入ります）。{SOURCE_NOTE}。
          </p>
        </>
      )}

      {budget && (
        <div className={finances.items.length > 0 ? "mt-3" : undefined}>
          <h2 className="text-sm font-semibold">
            この分野の今年の主な事業（{fiscalYearLabel(budget.fiscal_year)}
            ・予算）
          </h2>
          <ul className="mt-2 flex flex-col gap-1.5">
            {budget.projects.map((p) => (
              <li
                key={`${p.name}-${p.page}`}
                className="flex items-baseline justify-between gap-3 text-sm"
              >
                <span>
                  {p.flag && (
                    <span
                      className={`mr-1 inline-block rounded px-1 text-xs ${BADGE_AMBER}`}
                    >
                      {FLAG_LABEL[p.flag]}
                    </span>
                  )}
                  {budget.source.url ? (
                    <a
                      href={`${budget.source.url}#page=${p.page}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="underline-offset-2 hover:underline"
                    >
                      {p.name}
                    </a>
                  ) : (
                    p.name
                  )}
                </span>
                <span className="shrink-0 tabular-nums text-zinc-600 dark:text-zinc-400">
                  {thousandYen(p.amount)}
                </span>
              </li>
            ))}
          </ul>
          {budget.project_count > budget.projects.length && (
            <p className="mt-1 text-xs text-zinc-500">
              ほか {budget.project_count - budget.projects.length} 件
            </p>
          )}
          <p className="mt-2 text-xs leading-5 text-zinc-500">
            事業の名前を押すと、原本（{budget.source.title}
            ）のそのページが開きます。
          </p>
        </div>
      )}

      <p className="mt-2 text-xs">
        <Link
          href={`/${slug}/finances`}
          className="text-zinc-600 underline underline-offset-2 dark:text-zinc-400"
        >
          まちのお金を見る
        </Link>
      </p>
    </section>
  );
}
