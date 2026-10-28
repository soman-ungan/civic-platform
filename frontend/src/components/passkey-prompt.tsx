// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { BUTTON_PRIMARY } from "@/components/ui/styles";
import { errorMessage } from "@/lib/api-client";
import {
  isCancelled,
  loginWithPasskey,
  passkeyErrorMessage,
  registerPasskey,
} from "@/lib/passkey";

/**
 * 意見を投稿する前の、パスキーの登録の案内（ADR 0006）。投票は匿名のままでよいが、投稿にはパスキーが要る。
 * 名前やメールアドレスは聞かない。登録すると、それまでの投票はそのまま引き継がれる
 */
export function PasskeyPrompt({
  onDone,
  onCancel,
}: {
  onDone: () => void;
  onCancel: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function run(action: () => Promise<unknown>, fallback: string) {
    setBusy(true);
    setError(null);
    try {
      await action();
      onDone();
    } catch (e) {
      if (!isCancelled(e)) {
        setError(passkeyErrorMessage(e, errorMessage(e, fallback)));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <p className="text-sm font-medium">
        意見を投稿するには、パスキーを作ります
      </p>
      <p className="mt-1 text-xs leading-5 text-zinc-500">
        この端末の指紋・顔・PIN
        で作る鍵です。名前やメールアドレスは要りません。これまでの投票はそのまま残ります。
      </p>
      {error && (
        <p role="status" className="mt-2 text-sm text-rose-600">
          {error}
        </p>
      )}
      <div className="mt-3 flex flex-col gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            run(() => registerPasskey(), "パスキーを作れませんでした")
          }
          className={`h-11 text-sm ${BUTTON_PRIMARY}`}
        >
          {busy ? "確認中…" : "パスキーを作る"}
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            // 投票がアカウントにまとまり、進み具合が変わるので読み込み直す
            run(async () => {
              await loginWithPasskey();
              window.location.reload();
            }, "ログインできませんでした")
          }
          className="h-11 rounded-full text-sm text-zinc-700 underline underline-offset-2 dark:text-zinc-300"
        >
          ほかの端末で作ったパスキーでログイン
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={onCancel}
          className="h-9 text-xs text-zinc-500"
        >
          やめる
        </button>
      </div>
    </div>
  );
}
