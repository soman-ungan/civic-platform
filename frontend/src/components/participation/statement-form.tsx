// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { PasskeyPrompt } from "@/components/auth/passkey-prompt";
import { BUTTON_PRIMARY } from "@/components/ui/styles";
import { ApiError, apiFetch, errorMessage } from "@/lib/api/client";
import type {
  PostPermission,
  PostedStatement,
  Rejection,
  SimilarStatement,
  VoteValue,
} from "@/lib/api/types/participation";
import { MAX_STATEMENT_CHARS, countChars } from "@/lib/statement";

import { VOTE_BUTTONS } from "./vote-buttons";

/** 非公開になったときの案内（理由の種類ごと。ADR 0007） */
const REJECTION_MESSAGES: Record<Rejection, string> = {
  harm: "この内容は公開できませんでした。表現を見直してもう一度お試しください。",
  person:
    "人ではなく、政策や事業についての意見にしてみませんか。公人の進退などは、リコールや選挙などの手続に委ねています。",
  official_name:
    "首長・議員の名前を入れずに、政策や事業についての意見にしてください。",
  not_votable:
    "ほかの人が賛成・反対できる、このトピックについての1つの意見にしてください。迷ったときは、投票でパスを選べます。",
};

type Message =
  | { kind: "ok" | "error"; text: string }
  | { kind: "duplicate"; similar: SimilarStatement };

/**
 * 「自分の意見を追加」。返信やコメントではなく、投票カードに並ぶ新しい意見として投稿する。
 * 投稿できるかは backend の `posting` を見る（パスキーの登録・分析を見られること・1トピック1件。ADR 0005・0006）
 */
export function StatementForm({
  topicBase,
  posting,
  onChanged,
}: {
  /** トピックの API のパス（`topicApi(slug, id)`） */
  topicBase: string;
  posting: PostPermission;
  /** 投稿・パスキーの登録・案内した意見への投票のあと（次の意見と `posting` を読み直す） */
  onChanged: () => void;
}) {
  const [open, setOpen] = useState(false);
  // パスキーの登録の案内を出しているか
  const [needsPasskey, setNeedsPasskey] = useState(false);
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [message, setMessage] = useState<Message | null>(null);

  const count = countChars(text.trim());
  const tooLong = count > MAX_STATEMENT_CHARS;

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (count === 0 || tooLong) return;
    setSending(true);
    setMessage(null);
    try {
      const posted = await apiFetch<PostedStatement>(
        `${topicBase}/statements`,
        { method: "POST", body: { body: text } },
      );
      if (posted.status === "duplicate") {
        // 保存されていないので、文はそのまま残して書き直せるようにする
        setMessage({ kind: "duplicate", similar: posted.similar });
      } else if (posted.status === "rejected") {
        const base = REJECTION_MESSAGES[posted.rejection];
        setMessage({
          kind: "error",
          text: posted.suggestion
            ? `${base} 例:「${posted.suggestion}」`
            : base,
        });
      } else {
        setText("");
        setOpen(false);
        setMessage({
          kind: "ok",
          text:
            posted.status === "approved"
              ? "投稿しました。投票カードに追加されました。"
              : "投稿しました。確認のあと公開されます。",
        });
        onChanged();
      }
    } catch (e) {
      if (e instanceof ApiError && e.code === "registration_required") {
        setNeedsPasskey(true);
        return;
      }
      setMessage({
        kind: "error",
        text: errorMessage(e, "投稿に失敗しました"),
      });
      if (e instanceof ApiError) onChanged();
    } finally {
      setSending(false);
    }
  }

  async function voteSimilar(similar: SimilarStatement, value: VoteValue) {
    try {
      await apiFetch(
        `${topicBase}/statements/${encodeURIComponent(similar.id)}/votes`,
        { method: "POST", body: { value } },
      );
      setText("");
      setOpen(false);
      setMessage({ kind: "ok", text: "投票しました。" });
      onChanged();
    } catch (e) {
      setMessage({
        kind: "error",
        text: errorMessage(e, "投票に失敗しました"),
      });
    }
  }

  function start() {
    setMessage(null);
    if (posting.status === "registration_required") {
      setNeedsPasskey(true);
    } else {
      setOpen(true);
    }
  }

  return (
    <section className="mt-6">
      {message?.kind === "duplicate" ? (
        <div
          role="status"
          className="mb-3 rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800"
        >
          <p className="text-sm font-medium">同じ意見がすでにあります</p>
          <blockquote className="mt-3 rounded-xl bg-zinc-50 p-3 text-sm leading-6 dark:bg-zinc-900">
            {message.similar.body}
          </blockquote>
          {message.similar.your_vote !== undefined ? (
            <p className="mt-3 text-xs leading-5 text-zinc-500">
              この意見には「{voteLabel(message.similar.your_vote)}
              」で投票済みです。言いたいことが違うなら、違いが分かるように書き直して投稿できます。
            </p>
          ) : (
            <>
              <p className="mt-3 text-xs leading-5 text-zinc-500">
                新しく足す代わりに、この意見に投票してください。言いたいことが違うなら、違いが分かるように書き直して投稿できます。
              </p>
              <div className="mt-3 grid grid-cols-3 gap-2">
                {VOTE_BUTTONS.map((b) => (
                  <button
                    key={b.value}
                    type="button"
                    onClick={() => void voteSimilar(message.similar, b.value)}
                    className={`h-11 rounded-xl text-sm font-semibold ${b.className}`}
                  >
                    {b.label}
                  </button>
                ))}
              </div>
            </>
          )}
        </div>
      ) : (
        message && (
          <p
            role="status"
            className={`mb-3 text-sm ${message.kind === "ok" ? "text-emerald-700 dark:text-emerald-400" : "text-rose-600"}`}
          >
            {message.text}
          </p>
        )
      )}

      {needsPasskey ? (
        <PasskeyPrompt
          onRegistered={() => {
            setNeedsPasskey(false);
            // 登録しても、分析を見られるまでは投稿できないことがある。`posting` を読み直してから開く
            onChanged();
          }}
          onCancel={() => setNeedsPasskey(false)}
        />
      ) : posting.status === "votes_required" ? (
        <PostingNote>
          ほかの人の意見にあと{posting.remaining}
          件投票すると、自分の意見を投稿できます。先に出ている意見を読んでから書くと、同じ意見が重なりにくくなります。
        </PostingNote>
      ) : posting.status === "limit_reached" ? (
        <PostingNote>
          このトピックには、もう意見を投稿しています（1トピック1件まで）。ほかの意見への投票は続けられます。
        </PostingNote>
      ) : !open ? (
        <>
          <button
            type="button"
            onClick={start}
            className="h-12 w-full rounded-full border border-dashed border-zinc-400 text-sm text-zinc-700 dark:border-zinc-600 dark:text-zinc-300"
          >
            ＋ 自分の意見を追加
          </button>
          {/* 最初の意見は議会での主張だけなので、ほかの視点を足してもらうよう呼びかける */}
          <p className="mt-2 text-center text-xs leading-5 text-zinc-500">
            まだ出ていない視点や、別の案はありませんか？
            新しい意見は、ほかの参加者にも優先して表示されます。
          </p>
        </>
      ) : (
        <form
          onSubmit={submit}
          className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800"
        >
          <label htmlFor="statement-body" className="text-sm font-medium">
            あなたの意見（{MAX_STATEMENT_CHARS}文字まで）
          </label>
          <p className="mt-1 text-xs text-zinc-500">
            他の人の意見への返信ではなく、みんなが賛成・反対できる1つの意見として書いてください。人ではなく、政策や事業についての意見にしてください。投稿できるのは1トピック1件です。
          </p>
          <p className="mt-1 text-xs text-zinc-500">
            投稿は公開前に AI（Anthropic の
            Claude）で確認します。送るのは本文とトピックの公開済みの内容（タイトル・説明・ほかの意見）だけで、あなたを識別する情報は送りません。
          </p>
          <textarea
            id="statement-body"
            value={text}
            onChange={(e) => setText(e.target.value)}
            rows={3}
            className="mt-2 w-full resize-none rounded-xl border border-zinc-200 bg-white p-3 text-base leading-6 dark:border-zinc-700 dark:bg-zinc-900"
          />
          <div className="mt-2 flex items-center justify-between">
            <span
              className={`text-xs ${tooLong ? "font-medium text-rose-600" : "text-zinc-500"}`}
            >
              {count} / {MAX_STATEMENT_CHARS}
            </span>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="h-11 rounded-full px-4 text-sm text-zinc-600 dark:text-zinc-400"
              >
                やめる
              </button>
              <button
                type="submit"
                disabled={count === 0 || tooLong || sending}
                className={`h-11 px-5 text-sm ${BUTTON_PRIMARY}`}
              >
                {sending ? "確認中…" : "投稿する"}
              </button>
            </div>
          </div>
        </form>
      )}
    </section>
  );
}

/** 票の名前（ボタンの文から絵文字を除いたもの） */
function voteLabel(value: VoteValue): string {
  return { 1: "賛成", [-1]: "反対", 0: "パス" }[value];
}

function PostingNote({ children }: { children: React.ReactNode }) {
  return (
    <p className="rounded-2xl border border-dashed border-zinc-300 p-4 text-center text-xs leading-5 text-zinc-500 dark:border-zinc-700">
      {children}
    </p>
  );
}
