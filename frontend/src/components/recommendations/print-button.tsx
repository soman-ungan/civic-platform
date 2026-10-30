// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { BUTTON_SECONDARY } from "@/components/ui/styles";

/** 印刷・PDF に保存（ブラウザの印刷。印刷のときはボタンやヘッダーを出さない） */
export function PrintButton() {
  return (
    <button
      type="button"
      onClick={() => window.print()}
      className={`h-9 px-4 text-sm print:hidden ${BUTTON_SECONDARY}`}
    >
      印刷・PDF に保存
    </button>
  );
}
