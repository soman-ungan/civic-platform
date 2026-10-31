// SPDX-License-Identifier: AGPL-3.0-only

//! 段階2: DB を見る・直す（list / show / edit / add / remove / publish / archive）。
//! 中身は `records`（収集先は `ingest::collect`）。ここでは引数を読み、表示だけをする

//! 動詞ごとに `list`・`show`・`edit`（edit / add / remove）・`publish`（publish / archive）に分ける。
//! 対象（`Target`）と引数はここで決める

mod edit;
mod list;
mod publish;
mod show;

use clap::{Args, ValueEnum};

pub use edit::{add, edit, remove};
pub use list::list;
pub use publish::publish;
pub use show::show;

/// 段階2の対象（画面やドキュメントで使っている、データのまとまり）
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// 予算（款別・性質別・歳入・主な事業）
    Budget,
    /// 決算カード（費目ごとの金額と人口）
    Settlement,
    /// 取り込んだ会議録と、そこから作ったトピック
    Minutes,
    /// トピック（出典の引用・タグ）
    Topic,
    /// 意見
    Statement,
    /// タグ（分野の語彙と地区）。トピックに付ける・外す
    Tag,
    /// 収集先（collector_sources）
    Source,
    /// 運営者（パスキーで登録したアカウントのうち、運営者にしたもの。ADR 0006）
    Operator,
    /// 公開待ちの下書き（決まった時刻に古い順に1件ずつ公開する。docs/plans/2026-10-30-timeline-flow.md）
    Queue,
    /// 自動公開の時刻（自治体ごと。1つで1日1件）
    Slot,
    /// 似ているトピックの組（-m）、またはトピックの中の似ている意見の組（--topic）。費用 0（process::similarity）
    Similar,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    target: Target,
    /// 自治体の URL の名前（例: otawara）で絞る
    #[arg(short, long)]
    municipality: Option<String>,
    /// 下書きだけ（topic）
    #[arg(long)]
    drafts: bool,
    /// 意見の状態で絞る（statement。カンマ区切り。既定は確認待ち flagged,pending）
    #[arg(long, value_delimiter = ',')]
    status: Vec<String>,
    /// トピックで絞る（statement）
    #[arg(long)]
    topic: Option<String>,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    target: Target,
    /// 自治体の URL の名前（budget・settlement）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 年度（西暦。budget・settlement）
    #[arg(short, long)]
    year: Option<i32>,
    /// id（minutes は document の id、topic・statement・source はそれぞれの id）
    #[arg(long)]
    id: Option<String>,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    target: Target,
    /// トピック・意見・時刻（slot）の id
    #[arg(long)]
    id: Option<String>,
    /// 日本時間の時刻（slot。例: 07:30）
    #[arg(long)]
    at: Option<String>,
    /// タイトル（topic）
    #[arg(long)]
    title: Option<String>,
    /// 説明（topic）
    #[arg(long)]
    description: Option<String>,
    /// 本文（statement。140文字以内。投票がある意見は直せない）
    #[arg(long)]
    body: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    target: Target,
    /// 対象のトピックの id
    #[arg(long)]
    topic: Option<String>,
    /// 意見の本文（statement。140文字以内）
    #[arg(long)]
    body: Option<String>,
    /// タグの id（tag。例: field-kosodate、092100-kurobane。list tag で見られる）・下書きのトピックの id（queue）
    #[arg(long)]
    id: Option<String>,
    /// 自治体の URL の名前（slot）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 日本時間の時刻（slot。例: 07:30）
    #[arg(long)]
    at: Option<String>,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    target: Target,
    /// 意見の id（statement）・タグの id（tag）・運営者のアカウントの id（operator。list operator で見られる）・
    /// 下書きのトピックの id（queue）・時刻の id（slot。list slot で見られる）
    #[arg(long)]
    id: Option<String>,
    /// 対象のトピックの id（tag）
    #[arg(long)]
    topic: Option<String>,
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    target: Target,
    /// 自治体の URL の名前（budget）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 年度（西暦。budget）
    #[arg(short, long)]
    year: Option<i32>,
    /// トピックの id（topic）
    #[arg(long)]
    id: Option<String>,
}
