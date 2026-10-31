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

import { apiFetch } from "@/lib/api/client";
import { AUTH_API } from "@/lib/api/paths";
import type { Me } from "@/lib/api/types/auth";
import { useApi } from "@/lib/api/use-api";

type Started<T> = { challenge_id: string; options: { publicKey: T } };

/** この端末で、今の参加者にパスキーを作る（それまでの投票はそのまま）。`operatorToken` は運営者の招待 */
export async function registerPasskey(operatorToken?: string): Promise<Me> {
  const started = await apiFetch<
    Started<PublicKeyCredentialCreationOptionsJSON>
  >(AUTH_API.registerStart, {
    method: "POST",
    // 招待からの登録なら、パスキーの名前に「運営者」と入る（運営者にするのは finish）
    body: operatorToken ? { operator_token: operatorToken } : undefined,
  });
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
  rememberPasskey();
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
  rememberPasskey();
  notifyAuthChanged();
  return me;
}

export async function logout(): Promise<void> {
  await apiFetch<void>(AUTH_API.logout, { method: "POST" });
  notifyAuthChanged();
}

/**
 * 退会（`DELETE /api/auth/me`）。パスキー・関わり方・登録の印を消す。票と意見は匿名のまま残る。
 * 端末のパスキーはサーバーからは消せないので、対応するブラウザには「もう使えない鍵」だと知らせる
 * （パスワードの管理の一覧から消える）。対応していなければ、端末の設定から消してもらう。
 * 端末にも消えたと知らせられたかを返す
 */
export async function deleteAccount(): Promise<boolean> {
  const { credential_ids } = await apiFetch<{ credential_ids: string[] }>(
    AUTH_API.me,
    { method: "DELETE" },
  );
  forgetPasskey();
  notifyAuthChanged();
  const signal = (
    window.PublicKeyCredential as
      | {
          signalUnknownCredential?: (o: {
            rpId: string;
            credentialId: string;
          }) => Promise<void>;
        }
      | undefined
  )?.signalUnknownCredential;
  if (!signal || credential_ids.length === 0) return false;
  const results = await Promise.allSettled(
    credential_ids.map((credentialId) =>
      signal.call(window.PublicKeyCredential, {
        // パスキーの rp_id は本番のドメイン（= この画面のホスト名）
        rpId: window.location.hostname,
        credentialId,
      }),
    ),
  );
  return results.every((r) => r.status === "fulfilled");
}

// この端末でパスキーを作ったか・ログインしたかの印。ログアウトしても残す（次に「ログイン」を先に案内するため）。
// サーバーはログアウトした端末のパスキーを知らず、ブラウザにも端末のパスキーを調べる手段が無いので、ここに持つ。
// 中身は印だけで、利用者の ID などは入れない。消えても案内の順番が変わるだけ
const HAS_PASSKEY_KEY = "civic-platform:has-passkey";

function rememberPasskey() {
  try {
    window.localStorage.setItem(HAS_PASSKEY_KEY, "1");
  } catch {
    // 保存できないブラウザ（プライベートモードなど）では、印を残さない
  }
}

function forgetPasskey() {
  try {
    window.localStorage.removeItem(HAS_PASSKEY_KEY);
  } catch {
    // 保存できないブラウザでは、もとから印が無い
  }
}

/** この端末でパスキーを作ったか、ログインしたことがあるか */
export function hasPasskeyOnDevice(): boolean {
  try {
    return window.localStorage.getItem(HAS_PASSKEY_KEY) === "1";
  } catch {
    return false;
  }
}

// 投票の前の「ログインして投票しますか」で「ログインせずに投票」を選んだか。断ったら、このタブを閉じるまで聞かない
const SKIP_LOGIN_KEY = "civic-platform:vote-without-login";

/**
 * 投票の前に、ログイン（またはパスキーを作ること）を案内するか。ログインしていなければ、パスキーの有無に因らず案内する。
 * 端末の印（`hasPasskeyOnDevice`）は Cookie などと一緒に消えることがあり、消えた人が自分からログインすることは少ないため。
 * ログインせずに投票すると、同じ人が分析の中で2人に数えられることがある
 */
export function shouldOfferLoginBeforeVote(me: Me | null): boolean {
  if (!me || me.registered) return false;
  try {
    return window.sessionStorage.getItem(SKIP_LOGIN_KEY) !== "1";
  } catch {
    return true;
  }
}

/** 「ログインせずに投票する」を選んだ */
export function skipLoginBeforeVote() {
  try {
    window.sessionStorage.setItem(SKIP_LOGIN_KEY, "1");
  } catch {
    // 保存できなければ、次の投票でもう一度聞く
  }
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
  if (e instanceof WebAuthnError && e.code === "ERROR_CEREMONY_ABORTED") {
    return true;
  }
  // 利用者が閉じた・時間切れは NotAllowedError。@simplewebauthn/browser は WebAuthnError に包み
  // （code は ERROR_PASSTHROUGH_SEE_CAUSE_PROPERTY）、name に元の名前を入れる
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
