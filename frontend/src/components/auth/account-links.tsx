// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { errorMessage } from "@/lib/api/client";
import {
  isCancelled,
  loginWithPasskey,
  logout,
  useMe,
} from "@/lib/auth/passkey";

/**
 * フッターの「パスキーでログイン」「ログアウト」（ADR 0006）。読むことと投票はログインなしでできるので、目立たせない。
 * ログイン・ログアウトのあとは、参加の状態が変わるのでページを読み込み直す。
 * 投稿フォームでパスキーを作ったときは、`useMe` が読み直して「ログイン中」になる
 */
export function AccountLinks() {
  const { data: me } = useMe();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function run(action: () => Promise<unknown>, fallback: string) {
    setBusy(true);
    setError(null);
    try {
      await action();
      window.location.reload();
    } catch (e) {
      if (!isCancelled(e)) {
        setError(errorMessage(e, fallback));
      }
      setBusy(false);
    }
  }

  if (!me) return null;
  return (
    <span className="flex items-center gap-2">
      {error && <span className="text-rose-600">{error}</span>}
      {me.registered && <span>ログイン中</span>}
      <button
        type="button"
        disabled={busy}
        onClick={() =>
          me.registered
            ? run(logout, "ログアウトできませんでした")
            : run(loginWithPasskey, "ログインできませんでした")
        }
        className="underline underline-offset-2 hover:text-zinc-800 dark:hover:text-zinc-200"
      >
        {me.registered ? "ログアウト" : "パスキーでログイン"}
      </button>
    </span>
  );
}
