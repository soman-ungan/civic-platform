// SPDX-License-Identifier: AGPL-3.0-only

// 管理画面で画面の上に固定するもの（見出し・自治体・タブは、スクロールの外の固定のヘッダー。console.tsx）

/**
 * 下書きのカードのタイトルと説明。カードの終わりまでスクロールの範囲の上（固定のヘッダーのすぐ下）に貼り付き、
 * カードと一緒に流れる（sticky は親の範囲の中でだけ貼り付く）。カードの余白（p-4）の分だけ左右に広げる
 */
export const STICKY_CARD_HEAD =
  "sticky top-0 z-20 -mx-4 border-b border-zinc-200 bg-background/95 px-4 py-2 backdrop-blur dark:border-zinc-800";
