// SPDX-License-Identifier: AGPL-3.0-only

//! 段階1: 資料を DB に入れる（fetch → extract → import）。
//!
//! - `fetch`: 原本を取得して置き場（`data/private/fetched/<slug>/<対象>/`）に置く。会議録は規約のため手でコピーする
//! - `extract`: 原本を AI で読み、出力を `<名前>.extraction.json` に書く（費用がかかるのはここだけ）。
//!   `--manual` なら、共同作業（Claude Code など）のための指示文を書き出す（費用なし）
//! - `import`: 原本と出力を検査して DB に入れ、置き場の原本を消す（手でコピーした原本は消さない）
//!
//! 3つのコマンドは、原本の隣の記録（`<名前>.meta.json`）でつなぐ（`collect::store`）

use std::path::{Path, PathBuf};

use crate::ingest::collect::store::{Meta, sidecar};
use crate::ingest::collect::{self, fetch as fetch_cache, settlement_card};
use crate::ingest::extraction::Extraction;
use crate::ingest::{budget, minutes};
use clap::{Args, ValueEnum};

use super::{Ctx, municipality_id, required, skip};

/// 段階1の対象（資料の種類）
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// 決算カード（総務省。AI を使わない）
    Settlement,
    /// 当初予算の概要（市のサイトの PDF）
    Budget,
    /// 会議録（手でコピーしたテキスト）
    Minutes,
}

#[derive(Debug, Args)]
pub struct FetchArgs {
    target: Target,
    /// 自治体の URL の名前（例: otawara）。無ければ、その資料の収集先すべて
    #[arg(short, long)]
    municipality: Option<String>,
    /// 1つの年度だけ（西暦）
    #[arg(short, long)]
    year: Option<i32>,
    /// 1つの収集先だけ（list source の id）
    #[arg(long)]
    source: Option<String>,
    /// 取得して確かめるだけで、置き場に置かない
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ExtractArgs {
    target: Target,
    /// 自治体の URL の名前（例: otawara）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 年度（西暦。予算）
    #[arg(short, long)]
    year: Option<i32>,
    /// 原本（予算は既定で置き場の <年度>.pdf。会議録は必須）
    #[arg(long)]
    file: Option<PathBuf>,
    /// 会議名（会議録）
    #[arg(long)]
    title: Option<String>,
    /// 開催日（会議録。YYYY-MM-DD）
    #[arg(long)]
    held_on: Option<String>,
    /// 原本のページの URL（画面の出典のリンク）
    #[arg(long)]
    url: Option<String>,
    /// API を呼ばず、共同作業のための指示文・出力の形・入力を書き出す（費用なし）
    #[arg(long, conflicts_with = "dry_run")]
    manual: bool,
    /// AI を呼ばずに、入力を確かめるだけ（会議録は発言者・名簿に無い首長と議員・議事日程）
    #[arg(long)]
    dry_run: bool,
    /// 指示文の書き出す先（--manual。既定は原本の隣の <名前>.prompt.md）
    #[arg(long, requires = "manual")]
    out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    target: Target,
    /// 自治体の URL の名前（例: otawara）。決算カードは、無ければ置き場のすべての自治体
    #[arg(short, long)]
    municipality: Option<String>,
    /// 年度（西暦。予算は必須、決算カードは1つの年度だけにする）
    #[arg(short, long)]
    year: Option<i32>,
    /// 原本（予算は既定で置き場の <年度>.pdf。会議録は必須）
    #[arg(long)]
    file: Option<PathBuf>,
    /// AI の出力（既定は原本の隣の <名前>.extraction.json）
    #[arg(long)]
    extraction: Option<PathBuf>,
    /// 会議名（会議録。既定は extract のときの値）
    #[arg(long)]
    title: Option<String>,
    /// 開催日（会議録。YYYY-MM-DD。既定は extract のときの値）
    #[arg(long)]
    held_on: Option<String>,
    /// 原本のページの URL（既定は fetch・extract のときの値）
    #[arg(long)]
    url: Option<String>,
    /// 検査の結果を出すだけで、保存しない
    #[arg(long)]
    dry_run: bool,
    /// 取り込み済みの会議録・公開済みの予算を作り直す（予算は下書きに戻る）
    #[arg(long)]
    again: bool,
    /// 一般質問・代表質問の無い会議録（採決だけの日など）も取り込む
    #[arg(long)]
    force: bool,
    /// 取り込んだあとも、置き場の原本を消さない
    #[arg(long)]
    keep: bool,
}

// ===== fetch =====

pub async fn fetch(ctx: &Ctx, args: FetchArgs) -> anyhow::Result<()> {
    let wanted = |s: &collect::Source| match args.target {
        Target::Settlement => s.adapter == "soumu_settlement_card",
        Target::Budget => s.kind == "budget" && s.adapter == "html_link_list",
        Target::Minutes => false,
    };
    if args.target == Target::Minutes {
        return skip(
            "fetch",
            args.target,
            "会議録は規約のため自動では取得しない（手でコピーして data/private/minutes/ に置く）",
        );
    }
    let Some(contact) = &ctx.config.collector_contact else {
        anyhow::bail!(
            "COLLECTOR_CONTACT が設定されていないため、取得できません（User-Agent に入れる運営者の連絡先）"
        );
    };
    let targets: Vec<_> = collect::sources(&ctx.pool)
        .await?
        .into_iter()
        .filter(|s| wanted(s))
        .filter(|s| args.municipality.as_ref().is_none_or(|m| s.slug == *m))
        .filter(|s| match &args.source {
            Some(id) => s.id == *id,
            None => s.is_enabled,
        })
        .collect();
    if targets.is_empty() {
        anyhow::bail!("取得する収集先がありません（list source で確かめてください）");
    }
    let options = collect::Options {
        dry_run: args.dry_run,
        fiscal_year: args.year,
    };
    let store = ctx.store();
    for source in targets {
        println!("== {} ({})", source.id, source.adapter);
        let report = collect::fetch_source(&ctx.pool, &source, contact, &options, &store).await?;
        print_report(&report);
    }
    Ok(())
}

fn print_report(report: &collect::Report) {
    for line in &report.lines {
        println!("  {line}");
    }
    for warning in &report.warnings {
        println!("  警告: {warning}");
    }
}

// ===== extract =====

pub async fn extract(ctx: &Ctx, args: ExtractArgs) -> anyhow::Result<()> {
    match args.target {
        Target::Settlement => skip(
            "extract",
            args.target,
            "決算カードは AI を使わない（import settlement が Excel を読む）",
        ),
        Target::Budget => extract_budget(ctx, args).await,
        Target::Minutes => extract_minutes(ctx, args).await,
    }
}

async fn extract_budget(ctx: &Ctx, args: ExtractArgs) -> anyhow::Result<()> {
    let usage = "extract budget -m <slug> -y <年度> [--file <PDF>] [--manual | --dry-run]";
    let slug = required(&args.municipality, usage)?;
    let year = *required(&args.year, usage)?;
    let file = budget_file(ctx, slug, year, args.file.clone());
    if !file.exists() {
        anyhow::bail!(
            "{} がありません（fetch budget で取得するか、--file で手で取った PDF を渡す）",
            file.display()
        );
    }
    let mut meta = Meta::read(&file)?;
    if args.url.is_some() {
        meta.url = args.url.clone();
    }

    if args.manual {
        let mut prompt = budget::prompt(&ctx.pool, year).await?;
        prompt.attachment = Some(file.display().to_string());
        meta.extraction = Some("manual".into());
        meta.write(&file)?;
        return write_prompt(&file, args.out, &prompt.render(), "budget");
    }
    let input = budget_input(ctx, slug, year, &file, &meta).await?;
    if args.dry_run {
        let pages = budget::pdftext::page_texts(&input.pdf)?;
        let chars: usize = pages.iter().map(|p| p.chars().count()).sum();
        println!(
            "{}: {} ページ（{chars} 文字）。AI は呼んでいません",
            file.display(),
            pages.len()
        );
        return Ok(());
    }
    let llm = ctx.llm("予算の概要を読めません（--manual は使えます）")?;
    let value = budget::api_output(&ctx.pool, llm, &ctx.config.extraction_model, &input).await?;
    let out = write_extraction(&file, &value)?;
    meta.extraction = Some("api".into());
    meta.write(&file)?;
    let checked = budget::extract(&ctx.pool, &Extraction::Saved(value), &input).await?;
    print!("{}", budget::render(&checked, input.url.as_deref()));
    println!(
        "AI の出力を書きました: {}。次: import budget -m {slug} -y {year}{}",
        out.display(),
        file_option(&args.file)
    );
    Ok(())
}

async fn extract_minutes(ctx: &Ctx, args: ExtractArgs) -> anyhow::Result<()> {
    let usage = "extract minutes -m <slug> --file <会議録> --title <会議名> --held-on <YYYY-MM-DD> [--url <URL>] [--manual | --dry-run]";
    let slug = required(&args.municipality, usage)?;
    let file = required(&args.file, usage)?;
    let mut meta = Meta::read(file)?;
    // 会議名・開催日は、渡されたものを記録し、import でも使う
    meta.title = args.title.clone().or(meta.title);
    meta.held_on = args.held_on.clone().or(meta.held_on);
    meta.url = args.url.clone().or(meta.url);
    let input = minutes_input(ctx, slug, file, &meta, usage).await?;

    if args.dry_run {
        return minutes_dry_run(ctx, &input).await;
    }
    if args.manual {
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
        let prompt = minutes::prompt(&ctx.pool, &input).await?;
        meta.extraction = Some("manual".into());
        meta.write(file)?;
        return write_prompt(file, args.out, &prompt.render(), "minutes");
    }
    let llm = ctx.llm("会議録を読めません（--dry-run・--manual は使えます）")?;
    let value = minutes::api_output(&ctx.pool, llm, &ctx.config.extraction_model, &input).await?;
    let out = write_extraction(file, &value)?;
    meta.extraction = Some("api".into());
    meta.write(file)?;
    print_minutes_preview(ctx, &Extraction::Saved(value), &input).await?;
    println!(
        "\nAI の出力を書きました: {}。次: import minutes -m {slug} --file {}",
        out.display(),
        file.display()
    );
    Ok(())
}

/// 共同作業のための指示文を書き出す
fn write_prompt(
    file: &Path,
    out: Option<PathBuf>,
    prompt: &str,
    target: &str,
) -> anyhow::Result<()> {
    let out = out.unwrap_or_else(|| sidecar(file, "prompt.md"));
    std::fs::write(&out, prompt)?;
    println!("書き出しました: {}", out.display());
    println!(
        "出力の JSON の置き場所: {}  → import {target} ... --dry-run で確かめる",
        sidecar(file, "extraction.json").display()
    );
    Ok(())
}

fn write_extraction(file: &Path, value: &serde_json::Value) -> anyhow::Result<PathBuf> {
    let out = sidecar(file, "extraction.json");
    std::fs::write(&out, serde_json::to_string_pretty(value)?)?;
    Ok(out)
}

// ===== import =====

pub async fn import(ctx: &Ctx, args: ImportArgs) -> anyhow::Result<()> {
    match args.target {
        Target::Settlement => import_settlement(ctx, args).await,
        Target::Budget => import_budget(ctx, args).await,
        Target::Minutes => import_minutes(ctx, args).await,
    }
}

async fn import_settlement(ctx: &Ctx, args: ImportArgs) -> anyhow::Result<()> {
    let slugs: Vec<String> = match &args.municipality {
        Some(slug) => vec![slug.clone()],
        None => {
            sqlx::query_scalar("SELECT slug FROM municipalities ORDER BY id")
                .fetch_all(&ctx.pool)
                .await?
        }
    };
    let store = ctx.store();
    let mut any = false;
    for slug in slugs {
        if store.originals(&slug, "settlement")?.is_empty() {
            continue;
        }
        any = true;
        println!("== {slug}");
        let report =
            settlement_card::import(&ctx.pool, &store, &slug, args.year, args.dry_run, args.keep)
                .await?;
        print_report(&report);
    }
    if !any {
        return skip(
            "import",
            args.target,
            "置き場に新しい決算カードが無い（fetch settlement で取得する。変わっていなければ置かれない）",
        );
    }
    Ok(())
}

async fn import_budget(ctx: &Ctx, args: ImportArgs) -> anyhow::Result<()> {
    let usage =
        "import budget -m <slug> -y <年度> [--file <PDF>] [--extraction <JSON>] [--dry-run]";
    let slug = required(&args.municipality, usage)?;
    let year = *required(&args.year, usage)?;
    let file = budget_file(ctx, slug, year, args.file.clone());
    if !file.exists() {
        if args.file.is_none() {
            return skip(
                "import",
                args.target,
                &format!(
                    "置き場に {year}年度の予算の概要が無い（fetch budget で取得するか、--file で手で取った PDF を渡す）"
                ),
            );
        }
        anyhow::bail!("{} がありません", file.display());
    }
    let mut meta = Meta::read(&file)?;
    if args.url.is_some() {
        meta.url = args.url.clone();
    }
    let extraction = load_extraction(&file, args.extraction.as_deref(), &meta, "budget")?;
    let input = budget_input(ctx, slug, year, &file, &meta).await?;
    let checked = budget::extract(&ctx.pool, &extraction, &input).await?;
    print!("{}", budget::render(&checked, input.url.as_deref()));
    if args.dry_run {
        println!("（--dry-run のため保存していません）");
        return Ok(());
    }
    budget::save_draft(&ctx.pool, &input, &checked, args.again, extraction.method()).await?;
    finish(ctx, &file, &meta, args.keep).await?;
    println!(
        "下書きを作りました。確認: show budget -m {slug} -y {year} → publish budget -m {slug} -y {year}"
    );
    Ok(())
}

async fn import_minutes(ctx: &Ctx, args: ImportArgs) -> anyhow::Result<()> {
    let usage = "import minutes -m <slug> --file <会議録> [--extraction <JSON>] [--title <会議名>] [--held-on <YYYY-MM-DD>] [--dry-run]";
    let slug = required(&args.municipality, usage)?;
    let file = required(&args.file, usage)?;
    let mut meta = Meta::read(file)?;
    meta.title = args.title.clone().or(meta.title);
    meta.held_on = args.held_on.clone().or(meta.held_on);
    meta.url = args.url.clone().or(meta.url);
    let input = minutes_input(ctx, slug, file, &meta, usage).await?;
    let extraction = load_extraction(file, args.extraction.as_deref(), &meta, "minutes")?;
    if args.dry_run {
        print_minutes_preview(ctx, &extraction, &input).await?;
        println!(
            "\n（--dry-run のため保存していません。最初の意見のモデレーションは、取り込むときに通します）"
        );
        return Ok(());
    }
    match minutes::import(
        &ctx.pool,
        &extraction,
        &ctx.moderation,
        &input,
        args.again,
        args.force,
    )
    .await?
    {
        minutes::Outcome::NotTopical(agenda) => {
            println!(
                "一般質問などトピックを作る議事が無いので、取り込みませんでした（取り込むときは --force）:"
            );
            for item in agenda {
                println!("  {item}");
            }
        }
        minutes::Outcome::NoUtterances => anyhow::bail!(
            "発言が見つかりません。会議録の見出しの形を確かめてください（extract minutes --dry-run）"
        ),
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
            finish(ctx, file, &meta, args.keep).await?;
            println!("確認: show topic --id <id> → publish topic --id <id>");
        }
    }
    Ok(())
}

/// 保存が済んだあと: 取得の記録を残し（次からの条件付き取得に使う）、置き場の原本を消す
async fn finish(ctx: &Ctx, file: &Path, meta: &Meta, keep: bool) -> anyhow::Result<()> {
    if let Some(cache) = &meta.cache {
        fetch_cache::remember(&ctx.pool, cache).await?;
    }
    if !keep && ctx.store().discard(file)? {
        println!("置き場の原本を消しました: {}", file.display());
    }
    Ok(())
}

/// AI の出力を読む。既定の場所（原本の隣）のものは、記録にある作り方（api / manual）をそのまま残す。
/// 別の場所から渡したものは、共同作業で作ったもの（manual）とみなす
fn load_extraction(
    file: &Path,
    explicit: Option<&Path>,
    meta: &Meta,
    target: &str,
) -> anyhow::Result<Extraction<'static>> {
    let default = sidecar(file, "extraction.json");
    let path = explicit.unwrap_or(&default);
    if !path.exists() {
        anyhow::bail!(
            "AI の出力 {} がありません。先に extract {target} を実行してください（共同作業なら extract {target} --manual）",
            path.display()
        );
    }
    let method = if path == default {
        meta.extraction.as_deref().unwrap_or("manual")
    } else {
        "manual"
    };
    Extraction::from_file(path, method)
}

// ===== 共通 =====

/// 予算の原本の場所（既定は置き場の <年度>.pdf）
fn budget_file(ctx: &Ctx, slug: &str, year: i32, file: Option<PathBuf>) -> PathBuf {
    file.unwrap_or_else(|| ctx.store().dir(slug, "budget").join(format!("{year}.pdf")))
}

fn file_option(file: &Option<PathBuf>) -> String {
    file.as_ref()
        .map(|f| format!(" --file {}", f.display()))
        .unwrap_or_default()
}

async fn budget_input(
    ctx: &Ctx,
    slug: &str,
    year: i32,
    file: &Path,
    meta: &Meta,
) -> anyhow::Result<budget::Input> {
    Ok(budget::Input {
        municipality_id: municipality_id(&ctx.pool, slug).await?,
        fiscal_year: year,
        url: meta.url.clone(),
        collector_source_id: meta.source_id.clone(),
        pdf: std::fs::read(file)
            .map_err(|e| anyhow::anyhow!("{} を読めません: {e}", file.display()))?,
    })
}

async fn minutes_input(
    ctx: &Ctx,
    slug: &str,
    file: &Path,
    meta: &Meta,
    usage: &str,
) -> anyhow::Result<minutes::Input> {
    Ok(minutes::Input {
        municipality_id: municipality_id(&ctx.pool, slug).await?,
        title: required(&meta.title, usage)?.clone(),
        held_on: required(&meta.held_on, usage)?.clone(),
        url: meta.url.clone(),
        text: std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("{} を読めません: {e}", file.display()))?,
    })
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
        println!("この会議録は取り込み済みです（document {id}）。作り直すときは import の --again");
    }
    Ok(())
}

/// AI の出力を、保存せずに検査に通して見せる
async fn print_minutes_preview(
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
    Ok(())
}
