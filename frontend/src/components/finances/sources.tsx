// SPDX-License-Identifier: AGPL-3.0-only

import { type FinanceView } from "@/lib/finances/view";
import { SOURCE_NOTE } from "@/lib/finances/money";
import type { Finances } from "@/lib/types";

/** まちのお金の出典（決算カードの「加工して作成」の表示と、予算の概要の原本） */
export function FinanceSources({
  municipalityName,
  finances: f,
  view: v,
}: {
  municipalityName: string;
  finances: Finances;
  view: FinanceView;
}) {
  const s = f.settlement;
  const b = f.budget;
  return (
    <footer className="mt-4 flex flex-col gap-2 text-xs leading-5 text-zinc-500">
      {s && (
        <p>
          {SOURCE_NOTE}（{s.source.title}
          {s.source.page_url && (
            <>
              、
              <a
                href={s.source.page_url}
                target="_blank"
                rel="noopener noreferrer"
                className="underline underline-offset-2"
              >
                総務省のページ
              </a>
            </>
          )}
          ）。1人あたりの額は、総額をその年度の1月1日の住民基本台帳の人口で割ったものです。
        </p>
      )}
      {b && (
        <p>
          予算は、{municipalityName}の「
          {b.source.url ? (
            <a
              href={b.source.url}
              target="_blank"
              rel="noopener noreferrer"
              className="underline underline-offset-2"
            >
              {b.source.title}
            </a>
          ) : (
            b.source.title
          )}
          」から、数字と事業の名前を取り出して作成しました（AI
          で読み取り、本文と照らし合わせて確かめています）。
          {v.kind === "budget" &&
            v.populationYear &&
            `1人あたりの額は、${v.populationYear}年度の人口で割ったものです（予算の年度の人口はまだ公表されていないため）。`}
        </p>
      )}
    </footer>
  );
}
