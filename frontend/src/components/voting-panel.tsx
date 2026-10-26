// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useEffect, useState } from "react";

import { RelationSheet } from "@/components/relation-sheet";
import { StatementForm } from "@/components/statement-form";
import { ApiError, apiFetch } from "@/lib/api-client";
import type { NextStatement, VoteValue } from "@/lib/types";

const VOTE_BUTTONS: { value: VoteValue; label: string; className: string }[] = [
  {
    value: 1,
    label: "賛成👍️",
    className: "bg-emerald-600 text-white active:bg-emerald-700",
  },
  {
    value: -1,
    label: "反対👎️",
    className: "bg-rose-600 text-white active:bg-rose-700",
  },
  {
    value: 0,
    label: "パス",
    className:
      "border border-zinc-300 bg-white text-zinc-700 active:bg-zinc-100 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300",
  },
];

/**
 * Polis 風の投票。意見を1件ずつカードで出し、賛成 / 反対 / パスで次へ進む。
 * コメント欄・リプライは設けない（Polis の原則）。
 */
export function VotingPanel({
  slug,
  topicId,
  onProgress,
}: {
  slug: string;
  topicId: string;
  /** 次の意見を読み込むたびに、投票の進捗を知らせる */
  onProgress?: (progress: NextStatement["progress"]) => void;
}) {
  const base = `/api/municipalities/${encodeURIComponent(slug)}/topics/${encodeURIComponent(topicId)}`;

  const [data, setData] = useState<NextStatement | null>(null);
  const [loadError, setLoadError] = useState(false);
  const [voteError, setVoteError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // 居住地属性の登録待ちの投票（登録が済んだら送る）
  const [pendingVote, setPendingVote] = useState<VoteValue | null>(null);

  const load = useCallback(async () => {
    try {
      const next = await apiFetch<NextStatement>(`${base}/next-statement`);
      setData(next);
      setLoadError(false);
      onProgress?.(next.progress);
    } catch {
      setLoadError(true);
    }
  }, [base, onProgress]);

  // 初回の読み込み。投票後の再読み込みはイベントハンドラーから load() を呼ぶ
  useEffect(() => {
    let ignore = false;
    apiFetch<NextStatement>(`${base}/next-statement`)
      .then((next) => {
        if (!ignore) {
          setData(next);
          setLoadError(false);
          onProgress?.(next.progress);
        }
      })
      .catch(() => {
        if (!ignore) setLoadError(true);
      });
    return () => {
      ignore = true;
    };
  }, [base, onProgress]);

  async function sendVote(statementId: string, value: VoteValue) {
    setBusy(true);
    setVoteError(null);
    try {
      await apiFetch(
        `${base}/statements/${encodeURIComponent(statementId)}/votes`,
        {
          method: "POST",
          body: { value },
        },
      );
      await load();
    } catch (e) {
      if (e instanceof ApiError && e.code === "relation_required") {
        setPendingVote(value);
      } else {
        setVoteError(e instanceof ApiError ? e.message : "投票に失敗しました");
      }
    } finally {
      setBusy(false);
    }
  }

  function vote(value: VoteValue) {
    if (!data?.statement || busy) return;
    if (data.needs_relation) {
      setPendingVote(value);
      return;
    }
    void sendVote(data.statement.id, value);
  }

  function onRelationSaved() {
    const value = pendingVote;
    setPendingVote(null);
    setData((d) => (d ? { ...d, needs_relation: false } : d));
    if (value !== null && data?.statement) {
      void sendVote(data.statement.id, value);
    }
  }

  if (loadError) {
    return (
      <div className="rounded-3xl border border-zinc-200 p-6 text-center dark:border-zinc-800">
        <p className="text-sm text-zinc-600 dark:text-zinc-400">
          意見を読み込めませんでした。
        </p>
        <button
          type="button"
          onClick={() => void load()}
          className="mt-3 h-11 rounded-full border border-zinc-300 px-5 text-sm dark:border-zinc-700"
        >
          再読み込み
        </button>
      </div>
    );
  }

  if (!data) {
    return (
      <div
        aria-hidden
        className="h-56 animate-pulse rounded-3xl bg-zinc-100 dark:bg-zinc-900"
      />
    );
  }

  const { statement, progress } = data;
  const percent =
    progress.total === 0
      ? 0
      : Math.round((progress.voted / progress.total) * 100);

  return (
    <section aria-label="意見への投票">
      <div className="mb-3 flex items-center gap-3 text-xs text-zinc-500">
        <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
          <div
            className="h-full rounded-full bg-zinc-900 transition-[width] dark:bg-zinc-100"
            style={{ width: `${percent}%` }}
          />
        </div>
        <span>
          {progress.voted} / {progress.total}
        </span>
      </div>

      {statement ? (
        <>
          <div aria-live="polite">
            <blockquote
              key={statement.id}
              className="flex min-h-48 items-center rounded-3xl border border-zinc-200 bg-white p-6 text-lg leading-8 shadow-sm motion-safe:animate-card-in dark:border-zinc-800 dark:bg-zinc-900"
            >
              {statement.body}
            </blockquote>
          </div>

          {voteError && (
            <p className="mt-3 text-sm text-rose-600">{voteError}</p>
          )}

          {/* 親指で押しやすいよう、画面下部に固定する */}
          <div className="sticky bottom-0 -mx-4 mt-4 bg-background/95 px-4 pt-3 pb-[max(1rem,env(safe-area-inset-bottom))] backdrop-blur">
            <div className="grid grid-cols-3 gap-2">
              {VOTE_BUTTONS.map((b) => (
                <button
                  key={b.value}
                  type="button"
                  disabled={busy}
                  onClick={() => vote(b.value)}
                  className={`h-14 rounded-2xl text-base font-semibold transition-opacity disabled:opacity-50 ${b.className}`}
                >
                  {b.label}
                </button>
              ))}
            </div>
          </div>
        </>
      ) : (
        <div className="rounded-3xl border border-zinc-200 p-6 text-center dark:border-zinc-800">
          <p className="font-semibold">
            {progress.total === 0
              ? "まだ意見がありません"
              : "すべての意見に投票しました"}
          </p>
          <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
            {progress.total === 0
              ? "最初の意見を投稿してみませんか？"
              : "ありがとうございます。新しい意見が増えたら、また投票できます。"}
          </p>
        </div>
      )}

      <StatementForm
        endpoint={`${base}/statements`}
        onPosted={() => void load()}
      />

      {pendingVote !== null && (
        <RelationSheet
          slug={slug}
          onClose={() => setPendingVote(null)}
          onSaved={onRelationSaved}
        />
      )}
    </section>
  );
}
