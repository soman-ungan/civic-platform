// SPDX-License-Identifier: AGPL-3.0-only

// 意見の投稿のきまり（backend と同じ）

/** 意見の最大文字数（backend の MAX_STATEMENT_CHARS と同じ） */
export const MAX_STATEMENT_CHARS = 140;

/** サーバーと同じく、Unicode のコードポイント単位で数える（絵文字も1文字） */
export function countChars(text: string): number {
  return [...text].length;
}
