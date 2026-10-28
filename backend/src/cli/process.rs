// SPDX-License-Identifier: AGPL-3.0-only

//! 段階3: DB の中身を判定・計算する（moderate / classify / analyze）。
//! どれも API サーバーが裏で自動で動かしている処理と同じ。コマンドは、それを今すぐ動かすためにある

use backend::{analysis, moderation, tagging};
use clap::{Args, ValueEnum};

use super::{Ctx, skip};

/// 段階3の対象（市民が参加するもの）
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    Topic,
    Statement,
}

#[derive(Debug, Args)]
pub struct ModerateArgs {
    target: Target,
    /// 人が判定する意見の id。無ければ、判定待ち（pending）の意見を AI でもう一度判定する
    #[arg(long)]
    id: Option<String>,
    /// 公開する（--id と一緒に）
    #[arg(long, requires = "id", conflicts_with = "reject")]
    approve: bool,
    /// 非公開にする（--id と一緒に）
    #[arg(long, requires = "id")]
    reject: bool,
}

#[derive(Debug, Args)]
pub struct ClassifyArgs {
    target: Target,
    /// 1つのトピックだけ（無ければ、タグの無いトピックすべて）
    #[arg(long)]
    id: Option<String>,
    /// AI が付けたタグだけを付け直す（運営者が付けたタグは残す）
    #[arg(long, requires = "id")]
    force: bool,
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    target: Target,
    /// 1つのトピックだけ（無ければ、投票が変わったトピック）
    #[arg(long)]
    id: Option<String>,
    /// 投票が変わっていなくても計算する
    #[arg(long)]
    force: bool,
}

pub async fn moderate(ctx: &Ctx, args: ModerateArgs) -> anyhow::Result<()> {
    match args.target {
        Target::Statement => match &args.id {
            Some(id) => {
                if !args.approve && !args.reject {
                    anyhow::bail!("使い方: moderate statement --id <id> --approve | --reject");
                }
                if !moderation::decide(&ctx.pool, id, args.approve).await? {
                    anyhow::bail!("意見が見つかりません: {id}");
                }
                println!("{id}: {}", if args.approve { "approve" } else { "reject" });
            }
            None => {
                let n = moderation::retry_pending(&ctx.pool, &ctx.moderation).await?;
                println!("{n} 件を判定しました");
            }
        },
        Target::Topic => skip(
            "moderate",
            args.target,
            "トピックは運営者が確かめてから公開している（市民からのトピックの提案ができたら、ここで判定する。ADR 0005）",
        )?,
    }
    Ok(())
}

pub async fn classify(ctx: &Ctx, args: ClassifyArgs) -> anyhow::Result<()> {
    match args.target {
        Target::Topic => {
            let Some(tagger) = &ctx.tagger else {
                anyhow::bail!("ANTHROPIC_API_KEY が設定されていないため、タグ付けできません");
            };
            match &args.id {
                Some(topic_id) => {
                    let outcome =
                        tagging::tag_topic(&ctx.pool, tagger, topic_id, args.force).await?;
                    println!("{topic_id}: {outcome:?}");
                }
                None => {
                    let n = tagging::run_untagged(&ctx.pool, tagger).await?;
                    println!("{n} 件のトピックをタグ付けしました");
                }
            }
        }
        Target::Statement => skip(
            "classify",
            args.target,
            "意見にはタグを付けない（タグはトピックに付ける）",
        )?,
    }
    Ok(())
}

pub async fn analyze(ctx: &Ctx, args: AnalyzeArgs) -> anyhow::Result<()> {
    match args.target {
        Target::Topic => match &args.id {
            Some(topic_id) => {
                let outcome = analysis::store::run_topic(&ctx.pool, topic_id, args.force).await?;
                tracing::info!(topic_id, ?outcome, "analysis finished");
            }
            None => {
                let n = analysis::store::run_all(&ctx.pool, args.force).await?;
                tracing::info!(topics = n, "analysis finished");
            }
        },
        Target::Statement => skip(
            "analyze",
            args.target,
            "分析はトピックごとに行う（analyze topic）",
        )?,
    }
    Ok(())
}
