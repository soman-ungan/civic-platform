// SPDX-License-Identifier: AGPL-3.0-only

import { NextResponse, type NextRequest } from "next/server";

import { apiInternalUrl } from "@/lib/api/base";
import { MUNICIPALITIES_API } from "@/lib/api/paths";
import type { Municipality } from "@/lib/types";

/**
 * トップ（/）へのアクセス時、公開中の自治体が1つだけならそのページへ直接リダイレクトする。
 * 複数ある・取得に失敗した場合は、そのまま一覧ページを表示する。
 */
export async function proxy(request: NextRequest) {
  try {
    const res = await fetch(`${apiInternalUrl()}${MUNICIPALITIES_API}`);
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
