// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useRef, useState } from "react";

import { ChipButton } from "@/components/ui/chip";
import { MinutesQuote } from "@/components/ui/quote";
import { ScrollRow } from "@/components/ui/scroll-row";
import type { OfficialDetail } from "@/lib/api/types/officials";
import type { Tag } from "@/lib/api/types/topics";
import { coverageText } from "@/lib/officials";

/**
 * 議員の記録（ADR 0008）: その人の発言が出典になったトピック（新しい会議の順）と、その人の発言の引用。
 * 分野のタグで絞り込む。評価・順位・市民の投票の結果は載せない。
 * 議員の名前（`header`）から分野のチップまでは、ページのスクロールに重ねて上に固定する
 * （入れ子のスクロール枠は作らない）
 */
export function OfficialRecord({
  slug,
  official,
  header,
}: {
  slug: string;
  official: OfficialDetail;
  /** 戻るリンク・役職・名前・任期（ページの側で作る） */
  header: React.ReactNode;
}) {
  const [selected, setSelected] = useState<string | null>(null);
  const top = useRef<HTMLDivElement>(null);

  function select(tagId: string | null) {
    setSelected(tagId);
    // 絞り込んだら一覧の先頭へ（固定の見出しの下から見えるように、ページの一番上に戻す）
    top.current?.closest("main")?.scrollTo({ top: 0 });
  }
  const fields = uniqueFields(official);
  const visible = selected
    ? official.topics.filter((t) => t.tags.some((g) => g.id === selected))
    : official.topics;

  if (official.topics.length === 0) {
    return (
      <>
        {header}
        <p className="mt-4 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
          {coverageText(official.coverage)}
          この中には、{official.name}
          氏の発言が出典になったトピックはありません。
        </p>
      </>
    );
  }

  return (
    <>
      <div
        ref={top}
        className="sticky top-0 z-10 -mx-4 border-b border-zinc-200 bg-[var(--background)] px-4 pb-2 dark:border-zinc-800"
      >
        {header}
        <p className="mt-2 text-xs leading-5 text-zinc-500">
          {official.name}
          氏の議会での発言が出典になったトピックです（新しい会議の順）。
          {coverageText(official.coverage)}
        </p>
        {fields.length > 0 && (
          <ScrollRow
            label="分野で絞り込む"
            className="mt-3 pb-1"
            listClassName="gap-2"
          >
            <li>
              <ChipButton
                size="md"
                active={selected === null}
                onClick={() => select(null)}
              >
                すべて（{official.topics.length}）
              </ChipButton>
            </li>
            {fields.map(({ tag, count }) => (
              <li key={tag.id}>
                <ChipButton
                  size="md"
                  active={selected === tag.id}
                  onClick={() => select(selected === tag.id ? null : tag.id)}
                >
                  #{tag.name}（{count}）
                </ChipButton>
              </li>
            ))}
          </ScrollRow>
        )}
      </div>
      <ul className="mt-3 flex flex-col gap-3">
        {visible.map((topic) => (
          <li
            key={topic.id}
            className="rounded-2xl border border-zinc-200 bg-white p-4 dark:border-zinc-800 dark:bg-zinc-900"
          >
            <Link
              href={`/${slug}/topics/${topic.id}`}
              className="font-semibold leading-snug underline-offset-2 hover:underline"
            >
              {topic.title}
            </Link>
            <p className="mt-1 flex flex-wrap gap-x-3 text-xs">
              {topic.tags.map((g) => (
                <span key={g.id} className="text-sky-700 dark:text-sky-400">
                  #{g.name}
                </span>
              ))}
              {topic.closed && (
                <span className="text-zinc-500">締め切りました</span>
              )}
            </p>
            <ul className="mt-3 flex flex-col gap-3">
              {topic.quotes.map((q, i) => (
                <li key={i}>
                  <MinutesQuote
                    quote={q.quote}
                    documentTitle={q.document_title}
                    heldOn={q.held_on}
                    url={q.url}
                  />
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
      <p className="mt-4 text-xs leading-5 text-zinc-500">
        引用は会議録からの抜粋です。発言の全体は会議録をご覧ください。
      </p>
    </>
  );
}

/** 分野のタグと、そのタグのトピックの数（多い順。同じなら名前の順） */
function uniqueFields(official: OfficialDetail): { tag: Tag; count: number }[] {
  const counts = new Map<string, { tag: Tag; count: number }>();
  for (const topic of official.topics) {
    for (const tag of topic.tags) {
      if (tag.kind !== "field") continue;
      const entry = counts.get(tag.id) ?? { tag, count: 0 };
      entry.count += 1;
      counts.set(tag.id, entry);
    }
  }
  return [...counts.values()].sort(
    (a, b) => b.count - a.count || a.tag.name.localeCompare(b.tag.name, "ja"),
  );
}
