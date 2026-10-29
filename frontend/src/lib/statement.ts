// SPDX-License-Identifier: AGPL-3.0-only

// 意見とトピックの提案の文字数のきまり（backend/src/shared/limits.rs と同じ）

/** 意見の最大文字数 */
export const MAX_STATEMENT_CHARS = 140;

/** トピックのタイトルの最大文字数 */
export const MAX_TITLE_CHARS = 60;

/** トピックの説明の最大文字数（議会のトピックの説明と同じ長さ） */
export const MAX_DESCRIPTION_CHARS = 140;

/** サーバーと同じく、Unicode のコードポイント単位で数える（絵文字も1文字） */
export function countChars(text: string): number {
  return [...text].length;
}
