// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { EmptyState } from "@/components/ui/empty-state";
import { LoadMore } from "@/components/ui/load-more";
import {
  BADGE_AMBER,
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorApi, operatorStatementApi } from "@/lib/api/paths";
import type { QueueStatement } from "@/lib/api/types/operator";
import { usePaged } from "@/lib/api/use-paged";
import {
  QUEUE_KIND_LABELS,
  STATEMENT_STATUS_LABELS,
  formatDateTime,
} from "@/lib/operator";

import { useAction } from "./use-action";

/**
 * 確認待ち（flagged）と判定待ち（pending）の意見（古い順。スクロールで続きを読む）。公開するか非公開にする。
 * 判定の基準は ADR 0007（主張や政治的な立場では判断しない）。投稿者が誰かは出ない
 */
export function Queue({ slug }: { slug: string }) {
  const queue = usePaged<QueueStatement>(`${operatorApi(slug)}/statements`);

  if (!queue.loaded) {
    return queue.error ? (
      <p className={ERROR_TEXT}>読み込めませんでした</p>
    ) : (
      <p className="text-sm text-zinc-500">読み込み中…</p>
    );
  }
  if (queue.done && queue.items.length === 0) {
    return <EmptyState>確認待ちの意見はありません</EmptyState>;
  }
  return (
    <>
      <p className="mb-3 text-xs leading-5 text-zinc-500">
        主張の内容や政治的な立場では判断しません。公人の政策・公務への批判は公開、人格攻撃・私生活・人物の評価や進退は非公開。迷ったら公開（ADR
        0007）
      </p>
      <ul className="flex flex-col gap-4">
        {queue.items.map((item) => (
          <li key={item.id}>
            <QueueCard
              slug={slug}
              item={item}
              onDone={(status) =>
                queue.update((list) =>
                  status === "pending" || status === "flagged"
                    ? list.map((s) => (s.id === item.id ? { ...s, status } : s))
                    : list.filter((s) => s.id !== item.id),
                )
              }
            />
          </li>
        ))}
      </ul>
      <LoadMore
        count={queue.items.length}
        done={queue.done}
        loading={queue.loading}
        error={queue.error}
        onLoad={queue.loadMore}
      />
    </>
  );
}

function QueueCard({
  slug,
  item,
  onDone,
}: {
  slug: string;
  item: QueueStatement;
  /** 判定のあとの状態（確認待ちのままなら一覧に残す） */
  onDone: (status: string) => void;
}) {
  const { busy, error, run } = useAction();
  const [notice, setNotice] = useState<string | null>(null);
  const api = operatorStatementApi(slug, item.id);

  async function decide(approve: boolean) {
    const ok = await run(
      () =>
        apiFetch(`${api}/${approve ? "approve" : "reject"}`, {
          method: "POST",
        }),
      "判定を保存できませんでした",
    );
    if (ok) onDone(approve ? "approved" : "rejected");
  }

  async function retry() {
    setNotice(null);
    await run(async () => {
      const { status } = await apiFetch<{ status: string }>(`${api}/retry`, {
        method: "POST",
      });
      if (status === "pending") {
        setNotice("まだ判定できませんでした（通信の失敗か費用の上限）");
      }
      onDone(status);
    }, "やり直せませんでした");
  }

  return (
    <article className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <p className="text-xs text-zinc-500">
        {item.topic_title}
        {item.topic_status === "draft" && "（下書き）"}
      </p>
      <p className="mt-1 flex flex-wrap items-center gap-1.5 text-xs">
        <span className={`rounded px-1.5 py-0.5 ${BADGE_AMBER}`}>
          {STATEMENT_STATUS_LABELS[item.status]}
        </span>
        <span className="text-zinc-500">
          {QUEUE_KIND_LABELS[item.kind]}・{formatDateTime(item.created_at)}
        </span>
      </p>
      <p className="mt-2 text-sm leading-6">{item.body}</p>
      {item.moderation_note && (
        <p className="mt-2 rounded-lg bg-zinc-50 px-3 py-2 text-xs leading-5 text-zinc-600 dark:bg-zinc-900 dark:text-zinc-400">
          理由: {item.moderation_note}
        </p>
      )}
      {item.basis.length > 0 && (
        <p className="mt-2 text-xs text-zinc-500">
          根拠: {item.basis.join("、")}
          {item.document && (
            <>
              {"（"}
              {item.document.url ? (
                <a
                  href={item.document.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="underline underline-offset-2"
                >
                  {item.document.title}
                </a>
              ) : (
                item.document.title
              )}
              {item.document.held_on && ` ${item.document.held_on}`}
              {"）"}
            </>
          )}
        </p>
      )}
      {error && <p className={`mt-2 ${ERROR_TEXT}`}>{error}</p>}
      {notice && <p className="mt-2 text-xs text-zinc-500">{notice}</p>}
      <div className="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() => decide(true)}
          className={`h-9 px-4 text-sm ${BUTTON_PRIMARY}`}
        >
          公開する
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => decide(false)}
          className={`h-9 px-4 text-sm ${BUTTON_SECONDARY}`}
        >
          非公開にする
        </button>
        {item.status === "pending" && (
          <button
            type="button"
            disabled={busy}
            onClick={retry}
            className={`h-9 px-4 text-sm ${BUTTON_SECONDARY}`}
          >
            AI でやり直す
          </button>
        )}
      </div>
    </article>
  );
}
