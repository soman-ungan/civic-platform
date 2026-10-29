// SPDX-License-Identifier: AGPL-3.0-only

// 「みんなの意見」で画面の上に固定する見出し。重ねる位置がずれないよう、高さと位置はここだけで決める

/** 1段目: 「みんなの意見」。高さを固定する（2段目はこの高さの下に貼り付く） */
export const STICKY_TITLE =
  "sticky top-0 z-30 -mx-4 flex h-14 items-center justify-between bg-background/95 px-4 backdrop-blur";

/** 2段目: 各パネルの見出し。パネルが画面から出たら一緒に流れる（sticky は親の範囲の中でだけ貼り付く） */
export const STICKY_PANEL_HEADING =
  "sticky top-14 z-20 -mx-4 flex min-h-10 items-center bg-background/95 px-4 py-2 font-semibold backdrop-blur";

/**
 * 3段目: グループごとの特徴の表の見出しの行。1段目（3.5rem）と、1行の2段目（2.5rem）の下に貼り付く。
 * 2段目より下に重ねる（表が終わって流れるときに、2段目の下をくぐる）
 */
export const STICKY_TABLE_HEAD = "sticky top-24 z-15";
