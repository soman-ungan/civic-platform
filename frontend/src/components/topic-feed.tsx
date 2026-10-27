// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useRef, useState } from "react";

import { SOURCE_LABELS } from "@/lib/labels";
import type { Participation, Tag, Topic } from "@/lib/types";

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
        <nav
          aria-label="タグで絞り込む"
          data-swipe-ignore
          className="-mx-4 shrink-0 overflow-x-auto px-4 pb-3"
        >
          <ul className="flex w-max gap-2">
            <li>
              <Chip active={selected === null} onClick={() => select(null)}>
                すべて
              </Chip>
            </li>
            {tags.map((tag) => (
              <li key={tag.id}>
                <Chip
                  active={selected === tag.id}
                  onClick={() => select(selected === tag.id ? null : tag.id)}
                >
                  #{tag.name}
                </Chip>
              </li>
            ))}
          </ul>
        </nav>
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
      </div>
    </>
  );
}

function Chip({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={`h-9 whitespace-nowrap rounded-full border px-3.5 text-sm transition-colors ${
        active
          ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
          : "border-zinc-300 bg-white text-zinc-700 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300"
      }`}
    >
      {children}
    </button>
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
    badge:
      "bg-amber-100 text-amber-900 dark:bg-amber-900/60 dark:text-amber-100",
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
  return (
    <Link
      href={`/${slug}/topics/${topic.id}`}
      className={`block rounded-2xl border p-4 shadow-sm ${style.card}`}
    >
      <div className="mb-1 flex items-center justify-between gap-2">
        <p className="text-xs text-zinc-500">
          {SOURCE_LABELS[topic.source_type]}
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
        </span>
      </div>
    </Link>
  );
}
