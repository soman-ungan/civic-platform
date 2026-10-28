// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import { apiFetch } from "./api-client";

/**
 * Client Component で GET の結果を持つ。`path` か `refreshKey` が変わると読み込み直し、
 * 画面を離れたあと（や、次の読み込みが始まったあと）に届いた古い応答は捨てる。
 * 失敗しても前のデータは残し、`error` を立てる。
 * `initial` はサーバー側で取ってきた最初のデータ（HTML に最初から入るので、JS を待たずに表示できる）。
 * あれば最初の読み込みはしない
 */
export function useApi<T>(
  path: string,
  refreshKey?: unknown,
  initial?: T | null,
) {
  const [state, setState] = useState<{ data: T | null; error: boolean }>({
    data: initial ?? null,
    error: false,
  });
  const skipFirst = useRef(initial != null);

  useEffect(() => {
    if (skipFirst.current) {
      skipFirst.current = false;
      return;
    }
    let ignore = false;
    apiFetch<T>(path)
      .then((data) => {
        if (!ignore) setState({ data, error: false });
      })
      .catch(() => {
        if (!ignore) setState((s) => ({ ...s, error: true }));
      });
    return () => {
      ignore = true;
    };
  }, [path, refreshKey]);

  /** 今すぐ読み込み直す（投票のあとなど、イベントハンドラーから呼ぶ） */
  const reload = useCallback(async () => {
    try {
      const data = await apiFetch<T>(path);
      setState({ data, error: false });
    } catch {
      setState((s) => ({ ...s, error: true }));
    }
  }, [path]);

  /** 手元のデータだけを直す（応答を待たずに画面を変えるとき） */
  const update = useCallback((f: (data: T | null) => T | null) => {
    setState((s) => ({ ...s, data: f(s.data) }));
  }, []);

  return { data: state.data, error: state.error, reload, update };
}
