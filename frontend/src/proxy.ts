// SPDX-License-Identifier: AGPL-3.0-only

import { NextResponse, type NextRequest } from "next/server";

import type { Municipality } from "@/lib/types";

const API_INTERNAL_URL =
  process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080";

/**
 * トップ（/）へのアクセス時、公開中の自治体が1つだけならそのページへ直接リダイレクトする。
 * 複数ある・取得に失敗した場合は、そのまま一覧ページを表示する。
 */
export async function proxy(request: NextRequest) {
  try {
    const res = await fetch(`${API_INTERNAL_URL}/api/municipalities`);
    if (res.ok) {
      const municipalities = (await res.json()) as Municipality[];
      if (municipalities.length === 1) {
        return NextResponse.redirect(
          new URL(`/${municipalities[0].slug}`, request.url),
        );
      }
    }
  } catch {
    // backend に届かないときは一覧ページ側でエラーを表示する
  }
  return NextResponse.next();
}

export const config = {
  matcher: "/",
};
