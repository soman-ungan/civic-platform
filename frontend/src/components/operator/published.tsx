// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import { EmptyState } from "@/components/ui/empty-state";
import { LoadMore } from "@/components/ui/load-more";
import {
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorApi, operatorTopicApi } from "@/lib/api/paths";
import type { OperatorTopic } from "@/lib/api/types/operator";
import { usePaged } from "@/lib/api/use-paged";
import { deadlineLabel } from "@/lib/labels";
import { endOfDateInput, formatDateTime, toDateInput } from "@/lib/operator";

import { useAction } from "./use-action";

/** 公開中のトピック（新しい順。スクロールで続きを読む）: 期限の変更・すぐ締め切る・取り下げ（ADR 0005） */
export function Published({ slug }: { slug: string }) {
  const topics = usePaged<OperatorTopic>(
    `${operatorApi(slug)}/topics?status=published`,
  );

  if (!topics.loaded) {
    return topics.error ? (
      <p className={ERROR_TEXT}>読み込めませんでした</p>
    ) : (
      <p className="text-sm text-zinc-500">読み込み中…</p>
    );
  }
  if (topics.done && topics.items.length === 0) {
    return <EmptyState>公開中のトピックはありません</EmptyState>;
  }
  return (
    <>
      <ul className="flex flex-col gap-4">
        {topics.items.map((topic) => (
          <li key={topic.id}>
            <PublishedCard
              slug={slug}
              topic={topic}
              onChange={(next) =>
                topics.update((list) =>
                  next.status === "published"
                    ? list.map((t) => (t.id === next.id ? next : t))
                    : list.filter((t) => t.id !== next.id),
                )
              }
            />
          </li>
        ))}
      </ul>
      <LoadMore
        count={topics.items.length}
        done={topics.done}
        loading={topics.loading}
        error={topics.error}
        onLoad={topics.loadMore}
      />
    </>
  );
}

function PublishedCard({
  slug,
  topic,
  onChange,
}: {
  slug: string;
  topic: OperatorTopic;
  onChange: (topic: OperatorTopic) => void;
}) {
  const api = operatorTopicApi(slug, topic.id);
  const [date, setDate] = useState(
    topic.closes_at ? toDateInput(topic.closes_at) : "",
  );
  const { busy, error, run } = useAction();

  /** 期限を変える。`closesAt` が無ければ今（すぐ締め切る） */
  async function setDeadline(closesAt?: string) {
    await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(`${api}/closes-at`, {
            method: "PUT",
            body: closesAt ? { closes_at: closesAt } : {},
          }),
        ),
      "期限を変えられませんでした",
    );
  }

  async function closeNow() {
    if (
      !window.confirm(
        `「${topic.title}」を今すぐ締め切ります。投票と投稿が止まります（結果は読めます）。よろしいですか`,
      )
    )
      return;
    await setDeadline();
  }

  async function archive() {
    if (
      !window.confirm(
        `「${topic.title}」を取り下げます。結果も見られなくなります。よろしいですか`,
      )
    )
      return;
    await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(`${api}/archive`, { method: "POST" }),
        ),
      "取り下げられませんでした",
    );
  }

  const label = deadlineLabel(topic) ?? "期限なし";
  return (
    <article className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <h2 className="text-sm font-semibold">
        <Link
          href={`/${slug}/topics/${topic.id}`}
          className="underline-offset-2 hover:underline"
        >
          {topic.title}
        </Link>
      </h2>
      <p className="mt-1 text-xs text-zinc-500">
        {label}
        {topic.closes_at && `（${formatDateTime(topic.closes_at)}まで）`}
        {topic.published_at && `・公開 ${formatDateTime(topic.published_at)}`}
      </p>
      <div className="mt-3 flex flex-wrap items-end gap-2">
        <label className="text-xs text-zinc-500">
          期限（その日の終わりまで）
          <input
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
            className="mt-1 block h-9 rounded-lg border border-zinc-300 bg-white px-2 text-sm dark:border-zinc-700 dark:bg-zinc-900"
          />
        </label>
        <button
          type="button"
          disabled={busy || !date}
          onClick={() => setDeadline(endOfDateInput(date))}
          className={`h-9 px-4 text-sm ${BUTTON_PRIMARY}`}
        >
          期限を変える
        </button>
      </div>
      {error && <p className={`mt-2 ${ERROR_TEXT}`}>{error}</p>}
      <div className="mt-3 flex flex-wrap gap-2">
        {!topic.closed && (
          <button
            type="button"
            disabled={busy}
            onClick={closeNow}
            className={`h-9 px-4 text-sm ${BUTTON_SECONDARY}`}
          >
            今すぐ締め切る
          </button>
        )}
        <button
          type="button"
          disabled={busy}
          onClick={archive}
          className={`h-9 px-4 text-sm ${BUTTON_SECONDARY}`}
        >
          取り下げる
        </button>
      </div>
    </article>
  );
}
