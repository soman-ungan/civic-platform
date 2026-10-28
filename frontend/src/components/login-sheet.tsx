// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import {
  PASSKEY_EXPLANATION,
  PasskeyChoices,
} from "@/components/passkey-choices";

/**
 * ログインしていない人が投票する前のボトムシート（ADR 0006。docs/plans/2026-10-28-phase6a-passkeys.md の「ログインの流れ」）。
 * 投票はログインなしでもできる。「ログインせずに投票」を選べば、このタブを閉じるまで聞かない
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
          {PASSKEY_EXPLANATION}
          ログインすると、ほかの端末での投票と合わせて1人として数えられ、意見も投稿できます。ログインしなくても投票できます。
        </p>
        <PasskeyChoices
          createLabel="パスキーを作って投票"
          onRegistered={onRegistered}
          secondary={{ label: "ログインせずに投票", onClick: onSkip }}
        />
      </div>
    </div>
  );
}
