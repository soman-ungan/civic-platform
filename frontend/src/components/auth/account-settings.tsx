// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { BUTTON_SECONDARY, ERROR_TEXT } from "@/components/ui/styles";
import { errorMessage } from "@/lib/api/client";
import { deleteAccount, logout, useMe } from "@/lib/auth/passkey";

/**
 * アカウントのページの中身: ログアウトと退会（ADR 0006）。
 * 退会では、パスキー・関わり方・登録の印を消す。票と意見は匿名のまま残る
 */
export function AccountSettings() {
  const { data: me, error: loadFailed } = useMe();
  const router = useRouter();
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** 退会したあと。端末にも知らせられたか */
  const [deleted, setDeleted] = useState<{ signalled: boolean } | null>(null);

  async function remove() {
    setBusy(true);
    setError(null);
    try {
      setDeleted({ signalled: await deleteAccount() });
    } catch (e) {
      setError(errorMessage(e, "アカウントを削除できませんでした"));
    } finally {
      setBusy(false);
    }
  }

  async function signOut() {
    setBusy(true);
    setError(null);
    try {
      await logout();
      // 参加の状態が変わるので、サーバーで描いた分も読み直す
      router.push("/");
      router.refresh();
    } catch (e) {
      setError(errorMessage(e, "ログアウトできませんでした"));
      setBusy(false);
    }
  }

  if (deleted) {
    return (
      <div className="mt-4 flex flex-col gap-3">
        <p className="font-medium text-zinc-900 dark:text-zinc-100">
          アカウントを削除しました。
        </p>
        <p>
          {deleted.signalled
            ? "この端末のパスキーにも、使えなくなったことを知らせました。パスキーの一覧に残っていたら、端末の設定から消してください。"
            : "端末に残っているパスキーは、もう使えません。端末の設定（パスワードの管理など）から「civic-platform」のパスキーを消してください。"}
        </p>
        <Link href="/" className="underline underline-offset-2">
          トップへ
        </Link>
      </div>
    );
  }
  if (!me && !loadFailed) {
    return <p className="mt-4 text-zinc-500">読み込み中…</p>;
  }
  if (!me?.registered) {
    return (
      <div className="mt-4 flex flex-col gap-3">
        <p>
          この端末では、ログインしていません。読むことと投票は、ログインなしでできます。
        </p>
        <p>
          パスキーを作ったアカウントを削除するときは、フッターの「パスキーでログイン」から
          ログインしてから、このページを開いてください。
        </p>
      </div>
    );
  }

  return (
    <div className="mt-4 flex flex-col gap-8">
      <section className="flex flex-col gap-3">
        <p>
          この端末は、パスキーでログインしています
          {me.operator && "（運営者）"}。
        </p>
        <button
          type="button"
          disabled={busy}
          onClick={() => void signOut()}
          className={`h-11 self-start px-5 text-sm ${BUTTON_SECONDARY}`}
        >
          ログアウト
        </button>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="font-semibold text-zinc-900 dark:text-zinc-100">
          アカウントの削除
        </h2>
        <ul className="list-disc space-y-1 pl-5">
          <li>
            パスキーと、まちとの関わり方の答えを消します。元には戻せません
          </li>
          <li>
            それまでの投票と意見は、誰のものか分からないまま残ります（分析の結果が変わらないようにするため）。
            意見そのものを消したいときは、先に
            <Link
              href="/about#contact"
              className="underline underline-offset-2"
            >
              問い合わせ
            </Link>
            からご連絡ください
          </li>
          <li>
            削除したあとも、読むことと投票は、新しい参加者として続けられます
          </li>
          {me.operator && (
            <li className="font-medium text-rose-700 dark:text-rose-400">
              運営者ではなくなります。ほかに運営者がいなければ、CLI（add
              operator）で招待し直すまで管理画面を使えません
            </li>
          )}
        </ul>
        <label className="flex items-start gap-2">
          <input
            type="checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
            className="mt-1.5"
          />
          <span>上のことを確かめました</span>
        </label>
        {error && <p className={ERROR_TEXT}>{error}</p>}
        <button
          type="button"
          disabled={!confirmed || busy}
          onClick={() => void remove()}
          className="h-11 self-start rounded-full bg-rose-600 px-5 text-sm font-medium text-white disabled:opacity-40"
        >
          {busy ? "削除しています…" : "アカウントを削除する"}
        </button>
      </section>
    </div>
  );
}
