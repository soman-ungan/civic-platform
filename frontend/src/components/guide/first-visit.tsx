// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useParams, useRouter } from "next/navigation";
import { useEffect } from "react";

import { GUIDE_SEEN_KEY, guideHref } from "./links";

/**
 * 自治体のトップをその端末で初めて開いたときに、使い方のページ（/guide）を開く。
 * 印は localStorage だけ（サーバーには何も残さない。プライバシーポリシーに書いてある）。
 * 使えないとき（ブラウザの設定など）は開かない（毎回開かないように）。
 * 共有されたトピックの URL から直接来た人には出さない（自治体のトップの枠にだけ置く）
 */
export function GuideOnFirstVisit() {
  const router = useRouter();
  const { municipality } = useParams<{ municipality: string }>();

  useEffect(() => {
    try {
      if (localStorage.getItem(GUIDE_SEEN_KEY)) return;
      localStorage.setItem(GUIDE_SEEN_KEY, new Date().toISOString());
    } catch {
      return;
    }
    router.push(guideHref(municipality));
  }, [router, municipality]);

  return null;
}
