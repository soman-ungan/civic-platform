// SPDX-License-Identifier: AGPL-3.0-only

//! `eval`: 評価データ（evals/*.jsonl）を実際の API で判定し、結果を出す。費用がかかる

use std::path::PathBuf;

use crate::eval;
use crate::process::moderation::Moderation;
use crate::process::tagging::Tagger;
use clap::{Args, Subcommand};

use super::Ctx;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// モデレーションの評価（evals/moderation.jsonl）
    Moderation(Options),
    /// タグ付けの評価（evals/tagging.jsonl）
    Tagging(Options),
}

#[derive(Debug, Args)]
pub struct Options {
    /// モデル（既定は MODERATION_MODEL / TAGGING_MODEL）
    #[arg(long)]
    model: Option<String>,
    /// 先頭から何件だけ使うか
    #[arg(long)]
    limit: Option<usize>,
    /// 評価データのファイル
    #[arg(long)]
    file: Option<PathBuf>,
}

pub async fn run(ctx: &Ctx, command: Command) -> anyhow::Result<()> {
    let llm = ctx.llm("評価できません")?.clone();
    match command {
        Command::Moderation(o) => {
            let model = o.model.unwrap_or(ctx.config.moderation_model.clone());
            let file = o.file.unwrap_or("../evals/moderation.jsonl".into());
            let cases: Vec<eval::ModerationCase> = eval::load(&file)?;
            let cases = &cases[..cases.len().min(o.limit.unwrap_or(usize::MAX))];
            println!("モデレーションの評価: {model}、{} 件", cases.len());
            let moderation = Moderation::Llm { llm, model };
            let report = eval::run_moderation(&ctx.pool, &moderation, cases).await?;
            print!("{}", report.render());
        }
        Command::Tagging(o) => {
            let model = o.model.unwrap_or(ctx.config.tagging_model.clone());
            let file = o.file.unwrap_or("../evals/tagging.jsonl".into());
            let cases: Vec<eval::TaggingCase> = eval::load(&file)?;
            let cases = &cases[..cases.len().min(o.limit.unwrap_or(usize::MAX))];
            println!("タグ付けの評価: {model}、{} 件", cases.len());
            let tagger = Tagger { llm, model };
            let report = eval::run_tagging(&ctx.pool, &tagger, cases).await?;
            print!("{}", report.render());
        }
    }
    Ok(())
}
