// SPDX-License-Identifier: AGPL-3.0-only

// まとめで画面の上に固定する見出し。重ねる位置がずれないよう、高さと位置はここだけで決める
// （「みんなの意見」の components/analysis/sticky.ts と同じ考え方）

/** 1段目: トピックの題（1行に収め、高さを固定する）。そのトピックの節を表示している間だけ貼り付く */
export const STICKY_TOPIC =
  "sticky top-0 z-20 -mx-4 flex h-11 items-center gap-2 bg-background/95 px-4 backdrop-blur print:static";

/** 2段目: 節の見出し（合意点・意見が分かれた点・代表的な意見）。1段目（2.75rem）の下に貼り付き、節が終わると流れる */
export const STICKY_BLOCK =
  "sticky top-11 z-10 -mx-4 bg-background/95 px-4 py-1.5 backdrop-blur print:static";
