// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import {
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { errorMessage } from "@/lib/api-client";
import {
  hasPasskeyOnDevice,
  isCancelled,
  loginWithPasskey,
  passkeyErrorMessage,
  registerPasskey,
} from "@/lib/passkey";

/**
 * ログインしていない人が投票する前のボトムシート（ADR 0006）。
 * - パスキーがあればログイン（この端末の投票がアカウントにまとまる）
 * - 無ければパスキーを作る（今の参加者にパスキーが付き、そのまま投票する）
 * - ログインせずに投票（匿名のまま。このタブを閉じるまで聞かない）
 *
 * パスキーがあるかはサーバーにもブラウザにも分からないので、ログインと作るの両方を出し、
 * 端末の印（`hasPasskeyOnDevice`）があればログインを、無ければ作るのを主のボタンにする
 */
export function LoginSheet({
  onRegistered,
  onSkip,
  onClose,
}: {
  /** パスキーを作った（そのまま投票する） */
  onRegistered: () => void;
  /** ログインせずに投票する */
  onSkip: () => void;
  onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // シートは投票のボタンを押してから出るので、サーバーでの描画とずれない
  const [hasPasskey] = useState(hasPasskeyOnDevice);

  async function run(action: () => Promise<void>, fallback: string) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      if (!isCancelled(e)) {
        setError(passkeyErrorMessage(e, errorMessage(e, fallback)));
      }
      setBusy(false);
    }
  }

  const login = () =>
    run(async () => {
      await loginWithPasskey();
      // 投票がアカウントにまとまり、進み具合が変わるので読み込み直す
      window.location.reload();
    }, "ログインできませんでした");
  const create = () =>
    run(async () => {
      await registerPasskey();
      onRegistered();
    }, "パスキーを作れませんでした");

  const primary = hasPasskey
    ? { label: "パスキーでログイン", onClick: login }
    : { label: "パスキーを作って投票", onClick: create };
  const other = hasPasskey
    ? { label: "パスキーを作る（別のアカウントになります）", onClick: create }
    : { label: "作成済みのパスキーでログイン", onClick: login };

  return (
    <div
      className="fixed inset-0 z-50 flex items-end justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="login-title"
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-md rounded-t-3xl bg-white px-5 pt-5 pb-[max(1.25rem,env(safe-area-inset-bottom))] dark:bg-zinc-900"
      >
        <h2 id="login-title" className="text-lg font-bold">
          ログインして投票しますか
        </h2>
        <p className="mt-1 text-xs leading-5 text-zinc-500">
          パスキー（この端末の指紋・顔・PIN
          で作る鍵）を使います。名前やメールアドレスは要りません。ログインすると、ほかの端末での投票と合わせて1人として数えられ、意見も投稿できます。ログインしなくても投票できます。
        </p>

        {error && <p className={`mt-3 ${ERROR_TEXT}`}>{error}</p>}

        <div className="mt-5 flex flex-col gap-2">
          <button
            type="button"
            disabled={busy}
            onClick={() => void primary.onClick()}
            className={`h-12 text-sm ${BUTTON_PRIMARY}`}
          >
            {busy ? "確認中…" : primary.label}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={onSkip}
            className={`h-12 text-sm ${BUTTON_SECONDARY}`}
          >
            ログインせずに投票
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void other.onClick()}
            className="min-h-11 text-sm text-zinc-700 underline underline-offset-2 dark:text-zinc-300"
          >
            {other.label}
          </button>
        </div>
      </div>
    </div>
  );
}
