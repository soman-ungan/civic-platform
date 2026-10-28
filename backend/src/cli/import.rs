// SPDX-License-Identifier: AGPL-3.0-only

//! `import`: 手で取った会議録・予算の概要から下書きを作る。
//! AI の出力は API で得る（費用がかかる）か、共同作業で作った JSON を `--extraction` で渡す（費用なし）

use std::path::PathBuf;

use backend::extraction::Extraction;
use backend::{budget, minutes};
use clap::Subcommand;

use super::{Ctx, municipality_id};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 会議録からトピックの下書きを作る（1日分でおよそ 0.3〜0.4 ドル）
    Minutes {
        /// 自治体の URL の名前（例: otawara）
        #[arg(long)]
        municipality: String,
        /// 会議録の本文（UTF-8 のテキスト。data/private/ に置く）
        #[arg(long)]
        file: PathBuf,
        /// 会議名
        #[arg(long)]
        title: String,
        /// 開催日（YYYY-MM-DD）
        #[arg(long)]
        held_on: String,
        /// 会議録のページ（画面の出典のリンク）
        #[arg(long)]
        url: Option<String>,
        /// 保存しない。--extraction が無ければ、AI を呼ばずに発言者・名簿に無い首長と議員・議事日程だけを出す（費用なし）。
        /// あれば、その出力を取り込みと同じ検査に通した結果を出す
        #[arg(long)]
        dry_run: bool,
        /// 共同作業で作った AI の出力（JSON。extraction prompt の形）。API を呼ばない
        #[arg(long)]
        extraction: Option<PathBuf>,
        /// 取り込み済みの会議録を作り直す
        #[arg(long)]
        again: bool,
        /// 一般質問・代表質問の無い会議録も取り込む
        #[arg(long)]
        force: bool,
    },
    /// 当初予算の概要の PDF から予算の下書きを作る（40ページで約 0.8 ドル）
    Budget {
        /// 自治体の URL の名前（例: otawara）
        #[arg(long)]
        municipality: String,
        /// 当初予算の概要の PDF
        #[arg(long)]
        file: PathBuf,
        /// 年度（西暦）
        #[arg(long)]
        year: i32,
        /// 原本の PDF の URL（画面の「説明は原本 p.N」のリンク）
        #[arg(long)]
        url: Option<String>,
        /// 読んで確かめた結果を出すだけで、保存しない（--extraction が無ければ AI は呼ぶ）
        #[arg(long)]
        dry_run: bool,
        /// 共同作業で作った AI の出力（JSON。extraction prompt の形）。API を呼ばない
        #[arg(long)]
        extraction: Option<PathBuf>,
        /// 公開済みの年度を作り直す（下書きに戻る）
        #[arg(long)]
        again: bool,
    },
}

pub async fn run(ctx: &Ctx, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Minutes {
            municipality,
            file,
            title,
            held_on,
            url,
            dry_run,
            extraction,
            again,
            force,
        } => {
            let input = minutes::Input {
                municipality_id: municipality_id(&ctx.pool, &municipality).await?,
                title,
                held_on,
                url,
                text: std::fs::read_to_string(&file)?,
            };
            let extraction = match &extraction {
                Some(path) => Some(Extraction::manual_from_file(path)?),
                None => None,
            };
            match (dry_run, extraction) {
                (true, None) => minutes_dry_run(ctx, &input).await,
                (true, Some(extraction)) => minutes_preview(ctx, &extraction, &input).await,
                (false, Some(extraction)) => {
                    import_minutes(ctx, &extraction, &input, again, force).await
                }
                (false, None) => {
                    let extraction = Extraction::Api {
                        llm: ctx.llm("取り込めません（--dry-run・--extraction は使えます）")?,
                        model: &ctx.config.extraction_model,
                    };
                    import_minutes(ctx, &extraction, &input, again, force).await
                }
            }
        }
        Command::Budget {
            municipality,
            file,
            year,
            url,
            dry_run,
            extraction,
            again,
        } => {
            let extraction = match &extraction {
                Some(path) => Extraction::manual_from_file(path)?,
                None => Extraction::Api {
                    llm: ctx.llm("予算の概要を読めません（--extraction は使えます）")?,
                    model: &ctx.config.extraction_model,
                },
            };
            let input = budget::Input {
                municipality_id: municipality_id(&ctx.pool, &municipality).await?,
                fiscal_year: year,
                url,
                collector_source_id: None,
                pdf: std::fs::read(&file)?,
            };
            let checked = budget::extract(&ctx.pool, &extraction, &input).await?;
            print!("{}", budget::render(&checked, input.url.as_deref()));
            if dry_run {
                println!("（--dry-run のため保存していません）");
                return Ok(());
            }
            budget::save_draft(&ctx.pool, &input, &checked, again, extraction.method()).await?;
            println!(
                "下書きを作りました。確認: budget show {municipality} {year} → budget publish {municipality} {year}"
            );
            Ok(())
        }
    }
}

async fn minutes_dry_run(ctx: &Ctx, input: &minutes::Input) -> anyhow::Result<()> {
    let p = minutes::prepare(&ctx.pool, input).await?;
    println!(
        "発言 {} 件（{} 文字）",
        p.utterances.len(),
        input.text.chars().count()
    );
    println!(
        "議事日程: {}",
        if p.topical {
            "トピックを作る議事あり"
        } else {
            "形式的な議事だけ（取り込まない。取り込むときは --force）"
        }
    );
    for item in &p.agenda {
        println!("  {item}");
    }
    let mut speakers: Vec<(String, usize)> = Vec::new();
    for u in &p.utterances {
        let label = u.speaker_label();
        match speakers.iter_mut().find(|(l, _)| *l == label) {
            Some((_, n)) => *n += 1,
            None => speakers.push((label, 1)),
        }
    }
    for (label, n) in speakers {
        println!("  {label}: {n} 件");
    }
    if p.unmatched.is_empty() {
        println!("首長・議員はすべて名簿と対応付けられました");
    } else {
        println!("名簿に無い首長・議員（data/seeds/030_officials_*.sql に登録してください）:");
        for name in &p.unmatched {
            println!("  {name}");
        }
    }
    if let Some(id) = p.existing_document {
        println!("この会議録は取り込み済みです（document {id}）。作り直すときは --again");
    }
    Ok(())
}

/// 共同作業の出力を、保存せずに検査に通して見せる
async fn minutes_preview(
    ctx: &Ctx,
    extraction: &Extraction<'_>,
    input: &minutes::Input,
) -> anyhow::Result<()> {
    let (topics, notes) = minutes::preview(&ctx.pool, extraction, input).await?;
    println!("検査を通ったトピック案 {} 件", topics.len());
    for t in &topics {
        println!("\n## {}\n{}", t.title, t.description);
        for s in &t.sources {
            println!("  出典（発言 {}）:「{}」", s.utterance_no, s.quote);
        }
        for s in &t.statements {
            println!("  - {}（根拠: 発言 {:?}）", s.body, s.basis);
        }
    }
    if !notes.is_empty() {
        println!("\n捨てたもの:");
        for note in &notes {
            println!("  {note}");
        }
    }
    println!(
        "\n（--dry-run のため保存していません。最初の意見のモデレーションは、取り込むときに通します）"
    );
    Ok(())
}

async fn import_minutes(
    ctx: &Ctx,
    extraction: &Extraction<'_>,
    input: &minutes::Input,
    again: bool,
    force: bool,
) -> anyhow::Result<()> {
    match minutes::import(&ctx.pool, extraction, &ctx.moderation, input, again, force).await? {
        minutes::Outcome::NotTopical(agenda) => {
            println!(
                "一般質問などトピックを作る議事が無いので、取り込みませんでした（取り込むときは --force）:"
            );
            for item in agenda {
                println!("  {item}");
            }
        }
        minutes::Outcome::NoUtterances => {
            anyhow::bail!("発言が見つかりません。会議録の見出しの形を確かめてください（--dry-run）")
        }
        minutes::Outcome::Unmatched(names) => {
            println!("名簿に無い首長・議員がいるので止めました。登録してからやり直してください:");
            for name in names {
                println!("  {name}");
            }
        }
        minutes::Outcome::Duplicate(id) => {
            println!("この会議録は取り込み済みです（document {id}）。作り直すときは --again");
        }
        minutes::Outcome::Imported(report) => {
            println!(
                "発言 {} 件から、下書きを {} 件作りました",
                report.utterances,
                report.topics.len()
            );
            for (id, title) in &report.topics {
                println!("  {id}  {title}");
            }
            if !report.notes.is_empty() {
                println!("捨てたもの:");
                for note in &report.notes {
                    println!("  {note}");
                }
            }
            println!("確認: cargo run -- topics show <id> → topics publish <id>");
        }
    }
    Ok(())
}
