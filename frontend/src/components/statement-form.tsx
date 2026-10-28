// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { PasskeyPrompt } from "@/components/passkey-prompt";
import { BUTTON_PRIMARY } from "@/components/ui/styles";
import { ApiError, apiFetch, errorMessage } from "@/lib/api-client";
import { AUTH_API } from "@/lib/api-paths";
import { MAX_STATEMENT_CHARS, countChars } from "@/lib/statement";
import type { Me, PostedStatement } from "@/lib/types";
import { useApi } from "@/lib/use-api";

/**
 * 「自分の意見を追加」。返信やコメントではなく、投票カードに並ぶ新しい意見として投稿する。
 * 投稿にはパスキーの登録が要る（ADR 0006）。未登録なら、先に登録の案内を出す
 */
export function StatementForm({
  endpoint,
  onPosted,
}: {
  endpoint: string;
  onPosted: () => void;
}) {
  const [open, setOpen] = useState(false);
  // パスキーの登録の案内を出しているか
  const [needsPasskey, setNeedsPasskey] = useState(false);
  const { data: me, reload: reloadMe } = useApi<Me>(AUTH_API.me);
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
      const posted = await apiFetch<PostedStatement>(endpoint, {
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
      if (e instanceof ApiError && e.code === "registration_required") {
        setNeedsPasskey(true);
        return;
      }
      setMessage({
        kind: "error",
        text: errorMessage(e, "投稿に失敗しました"),
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

      {needsPasskey ? (
        <PasskeyPrompt
          onDone={async () => {
            await reloadMe();
            setNeedsPasskey(false);
            setOpen(true);
          }}
          onCancel={() => setNeedsPasskey(false)}
        />
      ) : !open ? (
        <>
          <button
            type="button"
            onClick={() => {
              setMessage(null);
              if (me?.registered) {
                setOpen(true);
              } else {
                setNeedsPasskey(true);
              }
            }}
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
            他の人の意見への返信ではなく、みんなが賛成・反対できる1つの意見として書いてください。
          </p>
          <p className="mt-1 text-xs text-zinc-500">
            投稿は公開前に AI（Anthropic の
            Claude）で確認します。送るのは本文だけで、あなたを識別する情報は送りません。
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
