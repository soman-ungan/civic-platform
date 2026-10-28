// SPDX-License-Identifier: AGPL-3.0-only

//! `topics`: 会議録から作ったトピックの下書きを確かめ、手直しし、公開する（ADR 0004）

use backend::status::Publication;
use backend::{drafts, moderation};
use clap::Subcommand;

use super::Ctx;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 下書きの一覧
    Drafts,
    /// 説明・出典の引用・最初の意見・タグ
    Show { id: String },
    /// 公開する
    Publish { id: String },
    /// 取り下げる
    Archive { id: String },
    /// タイトル・説明を直す
    Edit {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// 最初の意見を足す・直す・取り下げる（投票がある意見は直せない）
    Statement {
        #[command(subcommand)]
        command: StatementCommand,
    },
    /// タグを付ける・外す（例: field-kosodate、092100-kurobane）
    Tag {
        #[command(subcommand)]
        command: TagCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum StatementCommand {
    /// 意見を足す（140文字以内）
    Add { topic_id: String, body: String },
    /// 意見を直す
    Edit { id: String, body: String },
    /// 意見を取り下げる
    Remove { id: String },
}

#[derive(Debug, Subcommand)]
pub enum TagCommand {
    Add { topic_id: String, tag_id: String },
    Remove { topic_id: String, tag_id: String },
}

pub async fn run(ctx: &Ctx, command: Option<Command>) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match command.unwrap_or(Command::Drafts) {
        Command::Drafts => {
            let items = drafts::list(pool).await?;
            if items.is_empty() {
                println!("下書きのトピックはありません");
            }
            for d in items {
                println!(
                    "{}  {}  [{}]  {}",
                    d.id, d.created_at, d.municipality_id, d.title
                );
            }
        }
        Command::Show { id } => match drafts::show(pool, &id).await? {
            Some(detail) => print!("{}", detail.render()),
            None => anyhow::bail!("トピックが見つかりません: {id}"),
        },
        Command::Publish { id } => set_status(ctx, &id, Publication::Published).await?,
        Command::Archive { id } => set_status(ctx, &id, Publication::Archived).await?,
        Command::Edit {
            id,
            title,
            description,
        } => {
            if title.is_none() && description.is_none() {
                anyhow::bail!(
                    "使い方: topics edit <id> [--title <タイトル>] [--description <説明>]"
                );
            }
            drafts::edit_topic(pool, &id, title.as_deref(), description.as_deref()).await?;
            println!("{id}: 直しました");
        }
        Command::Statement { command } => match command {
            StatementCommand::Add { topic_id, body } => {
                let sid = drafts::add_statement(pool, &topic_id, &body).await?;
                println!("{sid}: 足しました");
            }
            StatementCommand::Edit { id, body } => {
                drafts::edit_statement(pool, &id, &body).await?;
                println!("{id}: 直しました");
            }
            StatementCommand::Remove { id } => {
                if !moderation::decide(pool, &id, false).await? {
                    anyhow::bail!("意見が見つかりません: {id}");
                }
                println!("{id}: 取り下げました");
            }
        },
        Command::Tag { command } => {
            let (topic_id, tag_id, add) = match command {
                TagCommand::Add { topic_id, tag_id } => (topic_id, tag_id, true),
                TagCommand::Remove { topic_id, tag_id } => (topic_id, tag_id, false),
            };
            drafts::set_tag(pool, &topic_id, &tag_id, add).await?;
            println!(
                "{topic_id}: {tag_id} を{}",
                if add {
                    "付けました"
                } else {
                    "外しました"
                }
            );
        }
    }
    Ok(())
}

async fn set_status(ctx: &Ctx, id: &str, status: Publication) -> anyhow::Result<()> {
    if !drafts::set_status(&ctx.pool, id, status).await? {
        anyhow::bail!("トピックが見つかりません: {id}");
    }
    println!("{id}: {}", status.as_str());
    Ok(())
}
