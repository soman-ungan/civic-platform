// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { EmptyState } from "@/components/ui/empty-state";
import { LoadMore } from "@/components/ui/load-more";
import { ERROR_TEXT } from "@/components/ui/styles";
import { operatorApi } from "@/lib/api/paths";
import type { OperatorTopic, TagChoice } from "@/lib/api/types/operator";
import { useApi } from "@/lib/api/use-api";
import { usePaged } from "@/lib/api/use-paged";

import { DraftCard } from "./draft-card";

/** 下書きのトピック（古い順。スクロールで続きを読む）。確かめて直し、公開待ちに入れるか、今すぐ公開するか、取り下げる */
export function Drafts({ slug }: { slug: string }) {
  const topics = usePaged<OperatorTopic>(`${operatorApi(slug)}/topics`);
  const tags = useApi<TagChoice[]>(`${operatorApi(slug)}/tags`);

  if (!topics.loaded) {
    return topics.error ? (
      <p className={ERROR_TEXT}>読み込めませんでした</p>
    ) : (
      <p className="text-sm text-zinc-500">読み込み中…</p>
    );
  }
  if (topics.done && topics.items.length === 0) {
    return <EmptyState>下書きのトピックはありません</EmptyState>;
  }
  return (
    <>
      <ul className="flex flex-col gap-6">
        {topics.items.map((topic) => (
          <li key={topic.id}>
            <DraftCard
              slug={slug}
              topic={topic}
              tagChoices={tags.data ?? []}
              onChange={(next) =>
                topics.update((list) =>
                  // 公開・取り下げ・公開待ちに入れたものは下書きの一覧から外す
                  next.status === "draft" && !next.queued_at
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
