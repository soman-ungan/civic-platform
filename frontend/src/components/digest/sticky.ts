// SPDX-License-Identifier: AGPL-3.0-only

// まとめで画面の上に固定する見出し（「みんなの意見」の components/analysis/sticky.ts と同じ考え方）。
// 1段目（トピックの題と説明）は説明の長さで高さが変わるので、測った高さを CSS の変数 --topic-head に入れ、
// 2段目はその下に1pxだけ重ねて貼り付ける。下を流れる中身が透けたり隙間から見えたりしないよう、背景は不透明にする。sticky は親の範囲の中でだけ貼り付くので、どちらもトピックの節（section）の直下に置く

/**
 * 1段目: トピックの題と説明。そのトピックの節を見ている間は上に残す。
 * スクロールする枠の上端より1px上に貼り付け（上端で1pxが隠れる）、枠の上端が端数の位置にあっても、
 * レイヤのつまみの行との間から下を流れる中身が見えないようにする（その1pxは上の余白で埋め合わせる）
 */
export const STICKY_TOPIC =
  "sticky -top-px z-20 -mx-4 bg-background px-4 pt-[calc(0.5rem+1px)] pb-2 print:static";

/** 2段目（下の「提案する」のボタン（z-30）より下に重ねる）: 節の見出し（合意点・意見が分かれた点・代表的な意見）。1段目の下に貼り付き、節が終わると流れる */
export const STICKY_BLOCK =
  "sticky top-[var(--topic-head,0px)] z-10 -mx-4 bg-background px-4 py-1.5 print:static";
