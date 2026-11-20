// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";

import { BUTTON_PRIMARY } from "@/components/ui/styles";

/** 使い方を一度出した印（localStorage）。プライバシーポリシーに書いてある */
export const GUIDE_SEEN_KEY = "cp_guide_seen";

/** 使い方のページへ。来た自治体を渡すと「はじめる」でそこへ戻る */
export function guideHref(slug?: string): string {
  return slug ? `/guide?from=${encodeURIComponent(slug)}` : "/guide";
}

/** 戻る先。自治体の URL の名前の形だけ受け付ける（ほかのサイトへ飛ばさない） */
function startHref(from: string | null): string {
  return from && /^[a-z0-9-]+$/.test(from) ? `/${from}` : "/";
}

/** 「はじめる」: 来た自治体のトップへ（無ければトップ。有効な自治体が1つならそこへ移る） */
export function StartButton() {
  const from = useSearchParams().get("from");
  return <StartLink href={startHref(from)} />;
}

export function StartLink({ href }: { href: string }) {
  return (
    <Link
      href={href}
      className={`flex h-12 w-full items-center justify-center text-base ${BUTTON_PRIMARY}`}
    >
      はじめる
    </Link>
  );
}
