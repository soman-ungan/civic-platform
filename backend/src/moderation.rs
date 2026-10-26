// SPDX-License-Identifier: AGPL-3.0-only

//! 投稿された意見のモデレーション。
//! フェーズ2は全件承認の仮実装。フェーズ4で AI 判定の分岐を追加する。

/// モデレーションの方式
#[derive(Debug, Clone, Copy, Default)]
pub enum Moderation {
    /// すべて承認する（AI モデレーション導入までの仮実装）
    #[default]
    AutoApprove,
}

/// 判定結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// 問題なし → 公開
    Approved,
    /// 有害・暴言など → 非公開。書き直しの提案を添えられる
    Rejected { suggestion: Option<String> },
    /// 判定に迷う → 管理者が確認する
    Flagged,
}

impl Verdict {
    /// statements.status に入れる値
    pub fn status(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected { .. } => "rejected",
            Self::Flagged => "flagged",
        }
    }
}

impl Moderation {
    pub async fn review(&self, _body: &str) -> Verdict {
        match self {
            Self::AutoApprove => Verdict::Approved,
        }
    }
}
