// SPDX-License-Identifier: AGPL-3.0-only

//! `extraction`: 共同作業（Claude Code など）で AI の作業をするための準備。
//! API に送るものと同じ指示文・出力の形・入力を1つの文書に書き出す。作った JSON は
//! `import ... --extraction <json>` で、API の時と同じ検査を通して取り込む

use std::path::{Path, PathBuf};

use backend::{budget, minutes};
use clap::Subcommand;

use super::{Ctx, municipality_id};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 指示文・出力の形・入力を書き出す（既定は原本の隣の <名前>.prompt.md）
    Prompt {
        #[command(subcommand)]
        command: PromptCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum PromptCommand {
    /// 会議録からトピック案を作る作業
    Minutes {
        /// 自治体の URL の名前（例: otawara）
        #[arg(long)]
        municipality: String,
        /// 会議録の本文（UTF-8 のテキスト）
        #[arg(long)]
        file: PathBuf,
        /// 会議名
        #[arg(long)]
        title: String,
        /// 開催日（YYYY-MM-DD）
        #[arg(long)]
        held_on: String,
        /// 書き出す先
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// 当初予算の概要を書き写す作業
    Budget {
        /// 当初予算の概要の PDF（作業では、この PDF を直接読む）
        #[arg(long)]
        file: PathBuf,
        /// 年度（西暦）
        #[arg(long)]
        year: i32,
        /// 書き出す先
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

pub async fn run(ctx: &Ctx, command: Command) -> anyhow::Result<()> {
    let Command::Prompt { command } = command;
    let (prompt, file, out) = match command {
        PromptCommand::Minutes {
            municipality,
            file,
            title,
            held_on,
            out,
        } => {
            let input = minutes::Input {
                municipality_id: municipality_id(&ctx.pool, &municipality).await?,
                title,
                held_on,
                url: None,
                text: std::fs::read_to_string(&file)?,
            };
            let prepared = minutes::prepare(&ctx.pool, &input).await?;
            if !prepared.unmatched.is_empty() {
                println!(
                    "注意: 名簿に無い首長・議員がいます（取り込みは止まります。data/seeds/030_officials_*.sql に登録してください）:"
                );
                for name in &prepared.unmatched {
                    println!("  {name}");
                }
            }
            if !prepared.topical {
                println!("注意: 形式的な議事だけの会議録です（取り込むときは --force）");
            }
            (minutes::prompt(&ctx.pool, &input).await?, file, out)
        }
        PromptCommand::Budget { file, year, out } => {
            let mut prompt = budget::prompt(&ctx.pool, year).await?;
            prompt.attachment = Some(file.display().to_string());
            (prompt, file, out)
        }
    };
    let out = out.unwrap_or_else(|| sibling(&file, "prompt.md"));
    std::fs::write(&out, prompt.render())?;
    println!("書き出しました: {}", out.display());
    println!(
        "出力の JSON の置き場所（例）: {}  → import ... --extraction <JSON> --dry-run で確かめる",
        sibling(&file, "extraction.json").display()
    );
    Ok(())
}

/// 原本の隣のファイル（`2026R08_0226.txt` → `2026R08_0226.prompt.md`）
fn sibling(file: &Path, suffix: &str) -> PathBuf {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    file.with_file_name(format!("{stem}.{suffix}"))
}
