// SPDX-License-Identifier: AGPL-3.0-only

//! 状態の値（DB の CHECK と同じ文字列）。Rust のコードではこの型で扱い、文字列の書き間違いを防ぐ。
//! SQL の絞り込み（`WHERE status = 'approved'` など）は読みやすさのため文字列のまま書き、
//! 値が CHECK とずれていないことはテスト（`tests/it/shared/schema.rs`）で確かめる

use serde::Serialize;

/// 意見の状態（`statements.status`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementStatus {
    /// 判定待ち（通信の失敗・費用の上限。あとで再判定する）
    Pending,
    /// 公開
    Approved,
    /// 非公開
    Rejected,
    /// 人が確かめる
    Flagged,
}

impl StatementStatus {
    pub const ALL: [Self; 4] = [Self::Pending, Self::Approved, Self::Rejected, Self::Flagged];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Flagged => "flagged",
        }
    }
}

/// 公開の状態（`topics.status`・`budgets.status`・`recommendations.status`）。下書きは API のどこにも出さない
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Publication {
    /// 運営者の確認待ち
    Draft,
    Published,
    /// 取り下げた
    Archived,
}

impl Publication {
    pub const ALL: [Self; 3] = [Self::Draft, Self::Published, Self::Archived];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
}

/// 提言の項目の種類（`recommendation_items.kind`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// 合意点（どのグループでも多くが賛成）
    Consensus,
    /// 意見が分かれた点（あるグループでは多くが賛成し、別のグループでは多くが反対）
    Divided,
}

impl ItemKind {
    pub const ALL: [Self; 2] = [Self::Consensus, Self::Divided];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Consensus => "consensus",
            Self::Divided => "divided",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}
