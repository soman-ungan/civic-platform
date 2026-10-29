// SPDX-License-Identifier: AGPL-3.0-only

//! トピックと意見の文字数の上限（文字数は Unicode のコードポイント単位）。
//! 会議録から作るトピックと、市民の提案（ADR 0005）で同じ値を使う

/// トピックのタイトル
pub const MAX_TITLE_CHARS: usize = 60;
/// トピックの説明（議会のトピックは AI が、市民の提案は本人が、この長さにまとめる）
pub const MAX_DESCRIPTION_CHARS: usize = 140;
/// 意見の本文（DB の CHECK と同じ）
pub const MAX_STATEMENT_CHARS: usize = 140;
