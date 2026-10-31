// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

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
/** フッターに失敗を出しておく時間 */
const ERROR_SHOWN_MS = 3000;

export function AccountLinks() {
  const { data: me } = useMe();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 失敗の表示は3秒で消し、いつものフッターに戻す（フッターは常に出ていて、狭いため）
  useEffect(() => {
    if (!error) return;
    const timer = window.setTimeout(() => setError(null), ERROR_SHOWN_MS);
    return () => window.clearTimeout(timer);
  }, [error]);

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
      {me.registered && (
        // ログアウトと退会はアカウントのページで
        <Link
          href="/account"
          className="underline underline-offset-2 hover:text-zinc-800 dark:hover:text-zinc-200"
        >
          ログイン中
        </Link>
      )}
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
