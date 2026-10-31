// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { EmptyState } from "@/components/ui/empty-state";

import type { OfficialList } from "@/lib/api/types/officials";
import { coverageText, roleLabel, termLabel } from "@/lib/officials";

/**
 * 議員の記録の一覧（ADR 0008）。首長が先、議員は読みの順（backend の並び。数では並べない）。
 * 数は「この人の発言が出典になったトピックの数」で、評価ではない
 */
export function OfficialListView({
  slug,
  municipalityName,
  list,
}: {
  slug: string;
  municipalityName: string;
  list: OfficialList;
}) {
  if (list.officials.length === 0) {
    return (
      <EmptyState>
        {municipalityName}の議員の名簿は、まだ登録していません。
      </EmptyState>
    );
  }
  const current = list.officials.filter((o) => o.term_end === null);
  const former = list.officials.filter((o) => o.term_end !== null);
  return (
    <>
      <p className="text-xs leading-5 text-zinc-500">
        議員と首長が議会で取り上げたことと、そこから生まれたトピックです。
        発言は会議録からの引用で、会議録へのリンクがあります。
        {coverageText(list.coverage)}
      </p>
      <OfficialLinks
        slug={slug}
        municipalityName={municipalityName}
        officials={current}
      />
      {former.length > 0 && (
        <>
          <h2 className="mt-6 mb-2 text-sm font-semibold">以前の議員</h2>
          <OfficialLinks
            slug={slug}
            municipalityName={municipalityName}
            officials={former}
          />
        </>
      )}
    </>
  );
}

function OfficialLinks({
  slug,
  municipalityName,
  officials,
}: {
  slug: string;
  municipalityName: string;
  officials: OfficialList["officials"];
}) {
  return (
    <ul className="mt-3 divide-y divide-zinc-200 rounded-2xl border border-zinc-200 bg-white dark:divide-zinc-800 dark:border-zinc-800 dark:bg-zinc-900">
      {officials.map((o) => (
        <li key={o.id}>
          <Link
            href={`/${slug}/council/${o.id}`}
            className="flex items-center justify-between gap-3 px-4 py-3 active:bg-zinc-50 dark:active:bg-zinc-800"
          >
            <span className="min-w-0">
              <span className="font-medium">{o.name}</span>
              <span className="ml-2 text-xs text-zinc-500">
                {roleLabel(municipalityName, o.role)}
              </span>
              <span className="block text-xs text-zinc-500">
                任期 {termLabel(o)}
              </span>
            </span>
            <span className="shrink-0 text-xs text-zinc-500">
              トピック {o.topic_count} 件 ›
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}
