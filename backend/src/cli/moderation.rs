// SPDX-License-Identifier: AGPL-3.0-only

//! `moderation`: 確認待ち（flagged）と判定待ち（pending）の意見を扱う（認証のフェーズ6までは CLI）

use backend::moderation;
use clap::Subcommand;

use super::Ctx;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 確認待ち（flagged）と判定待ち（pending）の意見
    List,
    /// 公開する
    Approve { id: String },
    /// 非公開にする
    Reject { id: String },
    /// 判定待ちの意見を、AI でもう一度判定する
    Retry,
}

pub async fn run(ctx: &Ctx, command: Option<Command>) -> anyhow::Result<()> {
    match command.unwrap_or(Command::List) {
        Command::List => {
            let items = moderation::queue(&ctx.pool).await?;
            if items.is_empty() {
                println!("確認待ちの意見はありません");
            }
            for item in items {
                println!(
                    "{}  [{}]  {}  topic={}\n    {}\n    理由: {}",
                    item.id,
                    item.status,
                    item.created_at,
                    item.topic_id,
                    item.body,
                    item.moderation_note.as_deref().unwrap_or("-"),
                );
            }
        }
        Command::Approve { id } => decide(ctx, &id, true).await?,
        Command::Reject { id } => decide(ctx, &id, false).await?,
        Command::Retry => {
            let n = moderation::retry_pending(&ctx.pool, &ctx.moderation).await?;
            println!("{n} 件を判定しました");
        }
    }
    Ok(())
}

async fn decide(ctx: &Ctx, id: &str, approve: bool) -> anyhow::Result<()> {
    if !moderation::decide(&ctx.pool, id, approve).await? {
        anyhow::bail!("意見が見つかりません: {id}");
    }
    println!("{id}: {}", if approve { "approve" } else { "reject" });
    Ok(())
}
