// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";

import { useMe } from "@/lib/auth/passkey";

/** ヘッダーの「管理画面」へのリンク。運営者にだけ出す（ほかの人には何も出さない） */
export function OperatorLink() {
  const { data: me } = useMe();
  if (!me?.operator) return null;
  return (
    <Link
      href="/operator"
      className="shrink-0 text-xs text-zinc-600 underline underline-offset-2 dark:text-zinc-400"
    >
      管理画面
    </Link>
  );
}
