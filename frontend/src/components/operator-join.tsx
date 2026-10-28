// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { BUTTON_PRIMARY } from "@/components/ui/styles";
import { errorMessage } from "@/lib/api-client";
import {
  isCancelled,
  passkeyErrorMessage,
  registerPasskey,
} from "@/lib/passkey";

/** 招待のリンクから、この端末でパスキーを作って運営者になる（ADR 0006。運営者は画面からは増やせない） */
export function OperatorJoin({ token }: { token: string | null }) {
  const [state, setState] = useState<"idle" | "busy" | "done">("idle");
  const [error, setError] = useState<string | null>(null);

  if (!token) {
    return (
      <p className="mt-4 text-sm text-rose-600">
        招待のリンクが正しくありません。運営者に新しいリンクをもらってください。
      </p>
    );
  }
  if (state === "done") {
    return (
      <p
        role="status"
        className="mt-4 text-sm text-emerald-700 dark:text-emerald-400"
      >
        運営者として登録しました。このリンクはもう使えません。
      </p>
    );
  }

  async function join() {
    setState("busy");
    setError(null);
    try {
      const me = await registerPasskey(token ?? undefined);
      if (me.operator) {
        setState("done");
        return;
      }
      setError(
        "運営者になれませんでした。運営者に新しいリンクをもらってください。",
      );
    } catch (e) {
      if (!isCancelled(e)) {
        setError(
          passkeyErrorMessage(e, errorMessage(e, "パスキーを作れませんでした")),
        );
      }
    }
    setState("idle");
  }

  return (
    <div className="mt-4">
      <p className="text-sm leading-6 text-zinc-700 dark:text-zinc-300">
        この端末でパスキーを作り、運営者として登録します。招待のリンクは1回だけ使えます。
      </p>
      {error && (
        <p role="status" className="mt-2 text-sm text-rose-600">
          {error}
        </p>
      )}
      <button
        type="button"
        disabled={state === "busy"}
        onClick={join}
        className={`mt-4 h-11 w-full text-sm ${BUTTON_PRIMARY}`}
      >
        {state === "busy" ? "確認中…" : "パスキーを作って運営者になる"}
      </button>
    </div>
  );
}
