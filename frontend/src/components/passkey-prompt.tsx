// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import {
  PASSKEY_EXPLANATION,
  PasskeyChoices,
} from "@/components/passkey-choices";

/**
 * 意見を投稿する前の、パスキーの案内（ADR 0006。docs/plans/2026-10-28-phase6a-passkeys.md の「ログインの流れ」）。
 * 投票はログインなしでもできるが、投稿にはパスキーが要る。作ると、それまでの投票はそのまま引き継がれる
 */
export function PasskeyPrompt({
  onRegistered,
  onCancel,
}: {
  /** パスキーを作った（そのまま投稿のフォームを開く） */
  onRegistered: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <p className="text-sm font-medium">
        意見を投稿するには、パスキーでのログインが要ります
      </p>
      <p className="mt-1 text-xs leading-5 text-zinc-500">
        {PASSKEY_EXPLANATION}これまでの投票はそのまま残ります。
      </p>
      <PasskeyChoices
        createLabel="パスキーを作って投稿"
        onRegistered={onRegistered}
        secondary={{ label: "やめる", onClick: onCancel }}
      />
    </div>
  );
}
