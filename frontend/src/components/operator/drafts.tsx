// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { EmptyState } from "@/components/ui/empty-state";
import { ERROR_TEXT } from "@/components/ui/styles";
import { operatorApi } from "@/lib/api/paths";
import type { OperatorTopic, TagChoice } from "@/lib/api/types/operator";
import { useApi } from "@/lib/api/use-api";

import { DraftCard } from "./draft-card";

/** 下書きのトピック（古い順）。確かめて直し、公開するか取り下げる */
export function Drafts({ slug }: { slug: string }) {
  const topics = useApi<OperatorTopic[]>(`${operatorApi(slug)}/topics`);
  const tags = useApi<TagChoice[]>(`${operatorApi(slug)}/tags`);

  if (topics.error && !topics.data) {
    return <p className={ERROR_TEXT}>読み込めませんでした</p>;
  }
  if (!topics.data) {
    return <p className="text-sm text-zinc-500">読み込み中…</p>;
  }
  if (topics.data.length === 0) {
    return <EmptyState>下書きのトピックはありません</EmptyState>;
  }
  return (
    <ul className="flex flex-col gap-6">
      {topics.data.map((topic) => (
        <li key={topic.id}>
          <DraftCard
            slug={slug}
            topic={topic}
            tagChoices={tags.data ?? []}
            onChange={(next) =>
              topics.update((list) =>
                // 公開・取り下げたものは下書きの一覧から外す
                next.status === "draft"
                  ? (list ?? []).map((t) => (t.id === next.id ? next : t))
                  : (list ?? []).filter((t) => t.id !== next.id),
              )
            }
          />
        </li>
      ))}
    </ul>
  );
}
