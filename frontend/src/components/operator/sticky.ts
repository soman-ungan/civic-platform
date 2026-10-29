// SPDX-License-Identifier: AGPL-3.0-only

// 管理画面で画面の上に固定するもの。重ねる位置がずれないよう、高さと位置はここだけで決める

/** 1段目: タブ（下書き・確認待ち・公開中）。高さを固定する（2段目はこの高さの下に貼り付く） */
export const STICKY_TABS =
  "sticky top-0 z-30 -mx-4 flex h-14 items-center border-b border-zinc-200 bg-background/95 px-4 backdrop-blur dark:border-zinc-800";

/**
 * 2段目: 下書きのカードのタイトルと説明。カードの終わりまで1段目の下に貼り付き、カードと一緒に流れる
 * （sticky は親の範囲の中でだけ貼り付く）。カードの余白（p-4）の分だけ左右に広げる
 */
export const STICKY_CARD_HEAD =
  "sticky top-14 z-20 -mx-4 border-b border-zinc-200 bg-background/95 px-4 py-2 backdrop-blur dark:border-zinc-800";
