// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import {
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { errorMessage } from "@/lib/api-client";
import { isCancelled, loginWithPasskey } from "@/lib/passkey";

/**
 * パスキーのある端末で、ログインせずに投票しようとしたときのボトムシート（ADR 0006）。
 * 投票はログインなしでもできるので、断れば（このタブを閉じるまで）そのまま投票できる
 */
export function LoginSheet({
  onSkip,
  onClose,
}: {
  /** ログインせずに投票する */
  onSkip: () => void;
  onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function login() {
    setBusy(true);
    setError(null);
    try {
      await loginWithPasskey();
      // 投票がアカウントにまとまり、進み具合が変わるので読み込み直す
      window.location.reload();
    } catch (e) {
      if (!isCancelled(e)) {
        setError(errorMessage(e, "ログインできませんでした"));
      }
      setBusy(false);
    }
  }

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
          パスキーでログインして投票しますか
        </h2>
        <p className="mt-1 text-xs leading-5 text-zinc-500">
          この端末にはパスキーがあります。ログインすると、これまでの投票と合わせて1人として数えられます。ログインしなくても投票できます。
        </p>

        {error && <p className={`mt-3 ${ERROR_TEXT}`}>{error}</p>}

        <div className="mt-5 flex gap-2">
          <button
            type="button"
            disabled={busy}
            onClick={onSkip}
            className={`h-12 flex-1 text-sm ${BUTTON_SECONDARY}`}
          >
            ログインせずに投票
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void login()}
            className={`h-12 flex-[2] text-sm ${BUTTON_PRIMARY}`}
          >
            {busy ? "確認中…" : "パスキーでログイン"}
          </button>
        </div>
      </div>
    </div>
  );
}
