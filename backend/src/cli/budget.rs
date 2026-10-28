// SPDX-License-Identifier: AGPL-3.0-only

//! `budget`: 予算の下書きの確認と公開。下書きは API のどこにも出ない

use backend::budget;
use backend::status::Publication;
use clap::Subcommand;

use super::{Ctx, municipality_id};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 予算の一覧（自治体・年度・状態）
    List,
    /// 款別・性質別・歳入・主な事業・捨てたもの
    Show { slug: String, year: i32 },
    /// 公開する
    Publish { slug: String, year: i32 },
    /// 取り下げる
    Archive { slug: String, year: i32 },
}

pub async fn run(ctx: &Ctx, command: Option<Command>) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match command.unwrap_or(Command::List) {
        Command::List => {
            let rows = budget::list(pool).await?;
            if rows.is_empty() {
                println!("予算はまだありません");
            }
            for (slug, year, status, total) in rows {
                println!("{slug}  {year}年度  [{status}]  歳出 {total} 千円");
            }
        }
        Command::Show { slug, year } => {
            let id = municipality_id(pool, &slug).await?;
            let Some((status, checked, url)) = budget::load(pool, &id, year).await? else {
                anyhow::bail!("{slug} の {year}年度の予算はありません");
            };
            println!("[{status}]");
            print!("{}", budget::render(&checked, url.as_deref()));
        }
        Command::Publish { slug, year } => {
            set_status(ctx, &slug, year, Publication::Published).await?
        }
        Command::Archive { slug, year } => {
            set_status(ctx, &slug, year, Publication::Archived).await?
        }
    }
    Ok(())
}

async fn set_status(ctx: &Ctx, slug: &str, year: i32, status: Publication) -> anyhow::Result<()> {
    let id = municipality_id(&ctx.pool, slug).await?;
    if !budget::set_status(&ctx.pool, &id, year, status).await? {
        anyhow::bail!("{slug} の {year}年度の予算はありません");
    }
    println!("{slug} {year}年度: {}", status.as_str());
    Ok(())
}
