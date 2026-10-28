// SPDX-License-Identifier: AGPL-3.0-only

//! `collect`: 行政資料の収集（docs/design/collection.md）。
//! 予算の概要は AI で読むので費用がかかる。--source で選ぶか --ai を付けたときだけ動かす

use backend::collect;
use clap::Subcommand;

use super::Ctx;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 収集先・規約の確認日・最後の取得
    List,
    /// 有効な収集先から取得する
    Run {
        /// 1つの収集先だけ
        #[arg(long)]
        source: Option<String>,
        /// 1つの年度だけ（西暦）
        #[arg(long)]
        year: Option<i32>,
        /// 保存しない（AI で読む収集先は、AI は呼ぶ）
        #[arg(long)]
        dry_run: bool,
        /// AI で資料を読む収集先（予算の概要）も動かす。費用がかかる
        #[arg(long)]
        ai: bool,
    },
}

pub async fn run(ctx: &Ctx, command: Option<Command>) -> anyhow::Result<()> {
    let sources = collect::sources(&ctx.pool).await?;
    match command.unwrap_or(Command::List) {
        Command::List => {
            if sources.is_empty() {
                println!("収集先はありません");
            }
            for s in sources {
                let terms = s
                    .config_json()
                    .ok()
                    .and_then(|c| c.get("terms_checked_at")?.as_str().map(str::to_owned));
                println!(
                    "{}  [{}] {} / {}  {}  規約の確認: {}  最後の取得: {}",
                    s.id,
                    s.municipality_id,
                    s.kind,
                    s.adapter,
                    if s.is_enabled { "有効" } else { "無効" },
                    terms.as_deref().unwrap_or("なし"),
                    s.last_crawled_at.as_deref().unwrap_or("-"),
                );
            }
        }
        Command::Run {
            source: only,
            year,
            dry_run,
            ai: with_ai,
        } => {
            let Some(contact) = &ctx.config.collector_contact else {
                anyhow::bail!(
                    "COLLECTOR_CONTACT が設定されていないため、収集できません（User-Agent に入れる運営者の連絡先）"
                );
            };
            let options = collect::Options {
                dry_run,
                fiscal_year: year,
            };
            let uses_ai = |s: &collect::Source| s.adapter == "html_link_list";
            let (targets, skipped): (Vec<_>, Vec<_>) = sources
                .into_iter()
                .filter(|s| match &only {
                    Some(id) => s.id == *id,
                    // 手動の取り込み（会議録など）は自動では取得しない
                    None => s.is_enabled && s.adapter != "manual_upload",
                })
                .partition(|s| only.is_some() || with_ai || !uses_ai(s));
            for s in &skipped {
                println!(
                    "{}: AI で資料を読む（費用がかかる）ので飛ばしました。動かすときは --source {} か --ai",
                    s.id, s.id
                );
            }
            if targets.is_empty() {
                anyhow::bail!("処理する収集先がありません（collect list で確かめてください）");
            }
            for source in targets {
                println!("== {} ({})", source.id, source.adapter);
                let ai = ctx.llm.as_ref().map(|llm| collect::Ai {
                    llm,
                    model: &ctx.config.extraction_model,
                });
                let report = collect::run_source(&ctx.pool, &source, contact, &options, ai).await?;
                for line in &report.lines {
                    println!("  {line}");
                }
                for warning in &report.warnings {
                    println!("  警告: {warning}");
                }
            }
        }
    }
    Ok(())
}
