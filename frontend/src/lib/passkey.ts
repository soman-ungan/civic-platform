// SPDX-License-Identifier: AGPL-3.0-only

// パスキー（WebAuthn）での登録とログイン（ADR 0006）。ブラウザの API の呼び出しは @simplewebauthn/browser に任せる。
// 名前やメールアドレスは使わない。backend は利用者の ID を返さず、登録済みか・運営者かだけを返す

"use client";

import {
  WebAuthnError,
  startAuthentication,
  startRegistration,
  type PublicKeyCredentialCreationOptionsJSON,
  type PublicKeyCredentialRequestOptionsJSON,
} from "@simplewebauthn/browser";

import { useEffect } from "react";

import { apiFetch } from "./api-client";
import { AUTH_API } from "./api-paths";
import type { Me } from "./types";
import { useApi } from "./use-api";

type Started<T> = { challenge_id: string; options: { publicKey: T } };

/** この端末で、今の参加者にパスキーを作る（それまでの投票はそのまま）。`operatorToken` は運営者の招待 */
export async function registerPasskey(operatorToken?: string): Promise<Me> {
  const started = await apiFetch<
    Started<PublicKeyCredentialCreationOptionsJSON>
  >(AUTH_API.registerStart, { method: "POST" });
  const credential = await startRegistration({
    optionsJSON: started.options.publicKey,
  });
  const me = await apiFetch<Me>(AUTH_API.registerFinish, {
    method: "POST",
    body: {
      challenge_id: started.challenge_id,
      credential,
      operator_token: operatorToken,
    },
  });
  notifyAuthChanged();
  return me;
}

/** ほかの端末で作ったパスキーでログインする（この端末で匿名で投票していた分は、アカウントにまとまる） */
export async function loginWithPasskey(): Promise<Me> {
  const started = await apiFetch<
    Started<PublicKeyCredentialRequestOptionsJSON>
  >(AUTH_API.loginStart, { method: "POST" });
  const credential = await startAuthentication({
    optionsJSON: started.options.publicKey,
  });
  const me = await apiFetch<Me>(AUTH_API.loginFinish, {
    method: "POST",
    body: { challenge_id: started.challenge_id, credential },
  });
  notifyAuthChanged();
  return me;
}

export async function logout(): Promise<void> {
  await apiFetch<void>(AUTH_API.logout, { method: "POST" });
  notifyAuthChanged();
}

// 登録・ログイン・ログアウトを、同じページのほかの部品（フッターなど）に知らせる
const AUTH_CHANGED = "civic-platform:auth-changed";

function notifyAuthChanged() {
  window.dispatchEvent(new Event(AUTH_CHANGED));
}

/**
 * ログインの状態（`GET /auth/me`）。登録・ログイン・ログアウトのあとは読み直す。
 * パスキーを作った時点で、その端末はそのアカウントでログインした状態になる
 */
export function useMe() {
  const result = useApi<Me>(AUTH_API.me);
  const { reload } = result;
  useEffect(() => {
    window.addEventListener(AUTH_CHANGED, reload);
    return () => window.removeEventListener(AUTH_CHANGED, reload);
  }, [reload]);
  return result;
}

/** ブラウザがパスキーの画面を閉じたとき（取り消し）は、エラーとして見せない */
export function isCancelled(e: unknown): boolean {
  if (e instanceof WebAuthnError) {
    return e.code === "ERROR_CEREMONY_ABORTED";
  }
  return e instanceof Error && e.name === "NotAllowedError";
}

/** パスキーの失敗を、画面に出す文にする */
export function passkeyErrorMessage(e: unknown, fallback: string): string {
  if (
    e instanceof WebAuthnError &&
    e.code === "ERROR_AUTHENTICATOR_PREVIOUSLY_REGISTERED"
  ) {
    return "この端末には、もうパスキーがあります。「パスキーでログイン」を使ってください。";
  }
  return fallback;
}
