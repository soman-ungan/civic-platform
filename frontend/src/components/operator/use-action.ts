// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useState } from "react";

import { errorMessage } from "@/lib/api/client";

/** 管理画面のボタンの処理: 実行中は押せなくし、失敗したら backend の文を出す */
export function useAction() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** 成功したら true */
  const run = useCallback(
    async (action: () => Promise<unknown>, fallback: string) => {
      setBusy(true);
      setError(null);
      try {
        await action();
        return true;
      } catch (e) {
        setError(errorMessage(e, fallback));
        return false;
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  return { busy, error, run };
}
