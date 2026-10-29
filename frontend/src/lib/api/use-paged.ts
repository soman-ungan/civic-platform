// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import { apiFetch } from "./client";

/** 分けて返す一覧（backend の `Page`。`next` があれば `?after=<next>` で続きを読む） */
export type Page<T> = { items: T[]; next: string | null };

/**
 * 分けて返す一覧を、続きを足しながら持つ（無限スクロール）。最初の1回は開いたときに読む。
 * `loadMore` は読み込み中・終わりなら何もしない。`update` は手元の一覧だけを直す（公開して外すときなど）
 */
export function usePaged<T>(path: string) {
  const [state, setState] = useState<{
    items: T[];
    next: string | null;
    loaded: boolean;
    done: boolean;
    error: boolean;
  }>({ items: [], next: null, loaded: false, done: false, error: false });
  const [loading, setLoading] = useState(true);
  // 読み込みの重なりを防ぐ（state の更新を待たずに分かるように ref で持つ）
  const busy = useRef(false);
  // 続きの位置（undefined はまだ1回も読めていない、null は終わり）
  const next = useRef<string | null | undefined>(undefined);
  const started = useRef(false);

  const load = useCallback(
    async (after: string | null) => {
      if (busy.current) return;
      busy.current = true;
      setLoading(true);
      try {
        const sep = path.includes("?") ? "&" : "?";
        const page = await apiFetch<Page<T>>(
          after ? `${path}${sep}after=${encodeURIComponent(after)}` : path,
        );
        next.current = page.next;
        setState((s) => ({
          items: after ? [...s.items, ...page.items] : page.items,
          next: page.next,
          loaded: true,
          done: page.next === null,
          error: false,
        }));
      } catch {
        setState((s) => ({ ...s, error: true }));
      } finally {
        busy.current = false;
        setLoading(false);
      }
    },
    [path],
  );

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void load(null);
  }, [load]);

  /** 続きを読む。最初の読み込みに失敗していたら、最初から読み直す */
  const loadMore = useCallback(() => {
    if (next.current === undefined) void load(null);
    else if (next.current !== null) void load(next.current);
  }, [load]);

  const update = useCallback((f: (items: T[]) => T[]) => {
    setState((s) => ({ ...s, items: f(s.items) }));
  }, []);

  return {
    items: state.items,
    /** 最初の読み込みが済んだか */
    loaded: state.loaded,
    done: state.done,
    loading,
    error: state.error,
    loadMore,
    update,
  };
}
