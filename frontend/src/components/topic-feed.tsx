// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import { SOURCE_LABELS } from "@/lib/labels";
import type { Tag, Topic } from "@/lib/types";

/** トピック一覧。タグのタップで、取得済みの一覧をその場で絞り込む（URL の ?tag= も更新） */
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

  function select(tagId: string | null) {
    setSelected(tagId);
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
          className="-mx-4 mb-4 overflow-x-auto px-4"
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

function TopicCard({ slug, topic }: { slug: string; topic: Topic }) {
  return (
    <Link
      href={`/${slug}/topics/${topic.id}`}
      className="block rounded-2xl border border-zinc-200 bg-white p-4 shadow-sm active:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:active:bg-zinc-800"
    >
      <p className="mb-1 text-xs text-zinc-500">
        {SOURCE_LABELS[topic.source_type]}
      </p>
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
