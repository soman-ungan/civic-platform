// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { ApiError, apiFetch } from "@/lib/api-client";
import { MAX_STATEMENT_CHARS, countChars } from "@/lib/labels";

type Posted = { id: string; status: string; suggestion?: string };

/** 「自分の意見を追加」。返信やコメントではなく、投票カードに並ぶ新しい意見として投稿する */
export function StatementForm({
  endpoint,
  onPosted,
}: {
  endpoint: string;
  onPosted: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [message, setMessage] = useState<{
    kind: "ok" | "error";
    text: string;
  } | null>(null);

  const count = countChars(text.trim());
  const tooLong = count > MAX_STATEMENT_CHARS;

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (count === 0 || tooLong) return;
    setSending(true);
    setMessage(null);
    try {
      const posted = await apiFetch<Posted>(endpoint, {
        method: "POST",
        body: { body: text },
      });
      if (posted.status === "approved") {
        setText("");
        setOpen(false);
        setMessage({
          kind: "ok",
          text: "投稿しました。投票カードに追加されました。",
        });
        onPosted();
      } else if (posted.status === "rejected") {
        setMessage({
          kind: "error",
          text: posted.suggestion
            ? `表現を見直してみませんか？ 例:「${posted.suggestion}」`
            : "この内容は公開できませんでした。表現を見直してもう一度お試しください。",
        });
      } else {
        setText("");
        setOpen(false);
        setMessage({
          kind: "ok",
          text: "投稿しました。確認のあと公開されます。",
        });
      }
    } catch (e) {
      setMessage({
        kind: "error",
        text: e instanceof ApiError ? e.message : "投稿に失敗しました",
      });
    } finally {
      setSending(false);
    }
  }

  return (
    <section className="mt-6">
      {message && (
        <p
          role="status"
          className={`mb-3 text-sm ${message.kind === "ok" ? "text-emerald-700 dark:text-emerald-400" : "text-rose-600"}`}
        >
          {message.text}
        </p>
      )}

      {!open ? (
        <button
          type="button"
          onClick={() => {
            setOpen(true);
            setMessage(null);
          }}
          className="h-12 w-full rounded-full border border-dashed border-zinc-400 text-sm text-zinc-700 dark:border-zinc-600 dark:text-zinc-300"
        >
          ＋ 自分の意見を追加
        </button>
      ) : (
        <form
          onSubmit={submit}
          className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800"
        >
          <label htmlFor="statement-body" className="text-sm font-medium">
            あなたの意見（{MAX_STATEMENT_CHARS}文字まで）
          </label>
          <p className="mt-1 text-xs text-zinc-500">
            他の人の意見への返信ではなく、みんなが賛成・反対できる1つの意見として書いてください。
          </p>
          <p className="mt-1 text-xs text-zinc-500">
            投稿は公開前に AI（Anthropic の Claude）で確認します。送るのは本文だけで、あなたを識別する情報は送りません。
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
                className="h-11 rounded-full bg-zinc-900 px-5 text-sm font-medium text-white disabled:opacity-40 dark:bg-zinc-100 dark:text-zinc-900"
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
