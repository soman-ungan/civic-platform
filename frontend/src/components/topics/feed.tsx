// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useRef, useState } from "react";

import { ChipButton } from "@/components/ui/chip";
import { ScrollRow } from "@/components/ui/scroll-row";
import { BADGE_AMBER } from "@/components/ui/styles";
import { deadlineLabel, SOURCE_LABELS } from "@/lib/labels";
import type { Participation } from "@/lib/api/types/participation";
import type { Tag, Topic } from "@/lib/api/types/topics";

/**
 * トピック一覧。タグのタップで、取得済みの一覧をその場で絞り込む（URL の ?tag= も更新）。
 * タグの列は固定し、カードの一覧だけがスクロールする（親は縦の flex で高さが決まっている）
 */
export function TopicFeed({
  slug,
  topics,
  tags,
  initialTag,
}: {
  slug: string;
  topics: Topic[];
  tags: Tag[];
  initialTag: string | null;
}) {
  const [selected, setSelected] = useState<string | null>(
    tags.some((t) => t.id === initialTag) ? initialTag : null,
  );
  const listRef = useRef<HTMLDivElement>(null);

  function select(tagId: string | null) {
    setSelected(tagId);
    listRef.current?.scrollTo({ top: 0 });
    const url = tagId
      ? `/${slug}?tag=${encodeURIComponent(tagId)}`
      : `/${slug}`;
    window.history.replaceState(null, "", url);
  }

  const visible = selected
    ? topics.filter((topic) => topic.tags.some((t) => t.id === selected))
    : topics;

  return (
    <>
      {tags.length > 0 && (
        <ScrollRow
          label="タグで絞り込む"
          className="shrink-0 pb-3"
          listClassName="gap-2"
        >
          <li>
            <ChipButton
              size="md"
              active={selected === null}
              onClick={() => select(null)}
            >
              すべて
            </ChipButton>
          </li>
          {tags.map((tag) => (
            <li key={tag.id}>
              <ChipButton
                size="md"
                active={selected === tag.id}
                onClick={() => select(selected === tag.id ? null : tag.id)}
              >
                #{tag.name}
              </ChipButton>
            </li>
          ))}
        </ScrollRow>
      )}

      <div
        ref={listRef}
        className="-mx-4 min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 pb-4"
      >
        {visible.length === 0 ? (
          <p className="py-8 text-center text-sm text-zinc-500">
            {topics.length === 0
              ? "まだトピックがありません。"
              : "このタグのトピックはありません。"}
          </p>
        ) : (
          <ul className="flex flex-col gap-3">
            {visible.map((topic) => (
              <li key={topic.id}>
                <TopicCard slug={slug} topic={topic} />
              </li>
            ))}
          </ul>
        )}
        {/* 市民からのトピックの提案（ADR 0005）。運営者が確かめてから公開する */}
        <Link
          href={`/${slug}/proposals`}
          className="mt-4 flex h-12 items-center justify-center rounded-full border border-dashed border-zinc-400 text-sm text-zinc-700 dark:border-zinc-600 dark:text-zinc-300"
        >
          ＋ トピックを提案する
        </Link>
      </div>
    </>
  );
}

/** 参加状態ごとのカードの見た目と、状態の表示 */
const PARTICIPATION_STYLES: Record<
  Participation,
  { card: string; badge: string | null }
> = {
  none: {
    card: "border-zinc-200 bg-white active:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:active:bg-zinc-800",
    badge: null,
  },
  in_progress: {
    card: "border-amber-400 bg-amber-50/60 active:bg-amber-100 dark:border-amber-600 dark:bg-amber-950/30",
    badge: BADGE_AMBER,
  },
  completed: {
    card: "border-emerald-200 bg-emerald-50/60 active:bg-emerald-100 dark:border-emerald-900 dark:bg-emerald-950/30",
    badge:
      "bg-emerald-100 text-emerald-900 dark:bg-emerald-900/60 dark:text-emerald-100",
  },
};

function participationLabel(topic: Topic): string | null {
  switch (topic.participation) {
    case "in_progress":
      return `参加中 ${topic.voted_count}/${topic.statement_count}`;
    case "completed":
      return "✓ 投票済み";
    default:
      return null;
  }
}

function TopicCard({ slug, topic }: { slug: string; topic: Topic }) {
  const style = PARTICIPATION_STYLES[topic.participation];
  const label = participationLabel(topic);
  const deadline = deadlineLabel(topic);
  return (
    <Link
      href={`/${slug}/topics/${topic.id}`}
      className={`block rounded-2xl border p-4 shadow-sm ${style.card}`}
    >
      <div className="mb-1 flex items-center justify-between gap-2">
        <p className="min-w-0 text-xs text-zinc-500">
          {topic.is_new && (
            <span className="mr-1.5 rounded-full bg-sky-700 px-1.5 py-0.5 text-[10px] font-bold tracking-wide text-white">
              NEW
            </span>
          )}
          {SOURCE_LABELS[topic.source_type]}
          {topic.session && ` ・ ${topic.session}`}
        </p>
        {label && style.badge && (
          <span
            className={`shrink-0 rounded-full px-2 py-0.5 text-xs font-medium ${style.badge}`}
          >
            {label}
          </span>
        )}
      </div>
      <h2 className="font-semibold leading-snug">{topic.title}</h2>
      {topic.description && (
        <p className="mt-1 line-clamp-2 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
          {topic.description}
        </p>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
        {topic.tags.map((tag) => (
          <span key={tag.id} className="text-sky-700 dark:text-sky-400">
            #{tag.name}
          </span>
        ))}
        <span className="ml-auto text-zinc-500">
          意見 {topic.statement_count} 件 ・ 参加 {topic.participant_count} 人
          {deadline && (
            // 日数はサーバーとブラウザの時刻で変わりうる
            <span suppressHydrationWarning> ・ {deadline}</span>
          )}
        </span>
      </div>
    </Link>
  );
}
