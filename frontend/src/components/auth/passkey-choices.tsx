// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import {
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { errorMessage } from "@/lib/api/client";
import {
  hasPasskeyOnDevice,
  isCancelled,
  loginWithPasskey,
  passkeyErrorMessage,
  registerPasskey,
} from "@/lib/auth/passkey";

/**
 * ログインしていない人への、パスキーの選択肢（ADR 0006）。投票の前（`LoginSheet`）と投稿の前（`PasskeyPrompt`）で使う。
 *
 * パスキーがあるかはサーバーにもブラウザにも分からないので、「ログイン」と「作る」を両方出し、
 * 端末の印（`hasPasskeyOnDevice`）があればログインを、無ければ作るのを主のボタンにする。
 * - ログイン: この端末の投票がアカウントにまとまり、進み具合が変わるので、ページを読み込み直す
 * - 作る: 今の参加者にパスキーが付く（投票はそのまま）。`onRegistered` で、元の操作を続ける
 */
export function PasskeyChoices({
  createLabel,
  onRegistered,
  secondary,
}: {
  /** 印が無いときの、作るボタンの文（「パスキーを作って投票」など） */
  createLabel: string;
  onRegistered: () => void;
  /** 2番目に出す、パスキーを使わない選択肢（「ログインせずに投票」「やめる」） */
  secondary: { label: string; onClick: () => void };
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // 選択肢はボタンを押してから出るので、サーバーでの描画とずれない
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
      window.location.reload();
    }, "ログインできませんでした");
  const create = () =>
    run(async () => {
      await registerPasskey();
      setBusy(false);
      onRegistered();
    }, "パスキーを作れませんでした");

  const primary = hasPasskey
    ? { label: "パスキーでログイン", onClick: login }
    : { label: createLabel, onClick: create };
  const other = hasPasskey
    ? { label: "パスキーを作る（別のアカウントになります）", onClick: create }
    : { label: "作成済みのパスキーでログイン", onClick: login };

  return (
    <>
      {error && <p className={`mt-3 ${ERROR_TEXT}`}>{error}</p>}
      <div className="mt-4 flex flex-col gap-2">
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
          onClick={secondary.onClick}
          className={`h-12 text-sm ${BUTTON_SECONDARY}`}
        >
          {secondary.label}
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
      <p className="mt-2 text-center text-xs leading-5 text-zinc-500">
        パスキーを作ると、
        <a
          href="/terms"
          target="_blank"
          className="underline underline-offset-2"
        >
          利用規約
        </a>
        と
        <a
          href="/privacy"
          target="_blank"
          className="underline underline-offset-2"
        >
          プライバシーポリシー
        </a>
        に同意したことになります。
      </p>
    </>
  );
}

/** パスキーの説明（投票の前・投稿の前で同じ） */
export const PASSKEY_EXPLANATION =
  "パスキーは、この端末の指紋・顔・PIN で作る鍵です。名前やメールアドレスは要りません。";
