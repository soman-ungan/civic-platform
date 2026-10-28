// SPDX-License-Identifier: AGPL-3.0-only

//! 運営のコマンド（`cargo run -- <動詞> <対象> [オプション]`）。使い方の全体は `docs/cli.md`、各コマンドは `--help`。
//! コマンドの処理はここに置き、中身（取り込み・判定・公開など）はライブラリのモジュールに任せる。
//!
//! 動詞は段階ごとに分ける（docs/plans/2026-10-28-cli-verbs.md）:
//! 1. 資料を DB に入れる（`ingest`: fetch / extract / import）
//! 2. DB を見る・直す（`records`: list / show / edit / add / remove / publish / archive）
//! 3. DB の中身を判定・計算する（`process`: moderate / classify / analyze）
//!
//! 段階に無い対象は clap が弾く。段階の中で、その対象には要らない処理は、何もせずに通す（`skip`）

mod eval;
mod ingest;
mod process;
mod records;

use std::path::Path;

use crate::ingest::collect::store::Store;
use crate::process::moderation::Moderation;
use crate::process::tagging::Tagger;
use crate::seed;
use crate::shared::config::Config;
use crate::shared::llm::Llm;
use crate::shared::llm::client::AnthropicClient;
use clap::{Parser, Subcommand};
use sqlx::SqlitePool;

#[derive(Debug, Parser)]
#[command(
    name = "backend",
    about = "civic-platform の API サーバーと、運営のコマンド。引数なしで API サーバーを起動する（docs/cli.md）"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    // ===== 1. 資料を DB に入れる =====
    /// 原本を取得して、置き場（data/private/fetched/）に置く
    Fetch(ingest::FetchArgs),
    /// 原本を AI で読み、出力（<名前>.extraction.json）を書く。保存はしない（API を使うと費用がかかる）
    Extract(ingest::ExtractArgs),
    /// 原本と AI の出力を検査して、DB に入れる（予算・会議録は下書き）。API は呼ばない
    Import(ingest::ImportArgs),

    // ===== 2. DB を見る・直す =====
    /// 一覧を出す
    List(records::ListArgs),
    /// 1件の中身を出す
    Show(records::ShowArgs),
    /// 直す
    Edit(records::EditArgs),
    /// 足す
    Add(records::AddArgs),
    /// 外す・取り下げる
    Remove(records::RemoveArgs),
    /// 下書きを公開する
    Publish(records::PublishArgs),
    /// 取り下げる（画面から消える。データは残る）
    Archive(records::PublishArgs),

    // ===== 3. DB の中身を判定・計算する（サーバーが裏で動かしている処理を、今すぐ動かす） =====
    /// 意見を判定する（AI で判定し直す、または人が承認・非公開にする）
    Moderate(process::ModerateArgs),
    /// トピックに分野・地区のタグを AI で付ける（ANTHROPIC_API_KEY が要る）
    Classify(process::ClassifyArgs),
    /// 意見の分析（意見マップ・グループ・合意点）を計算し直す
    Analyze(process::AnalyzeArgs),

    // ===== 準備と保守 =====
    /// リポジトリの seed（自治体・地区・分野タグ・公人の名簿・収集先）を入れる
    Seed {
        /// 続けて data/dev-seeds/（【サンプル】の架空のデータ）も入れる。本番では使わない
        #[arg(long)]
        dev: bool,
    },
    /// AI のモデレーション・タグ付けの精度を評価する（実際の API を呼ぶので費用がかかる）
    Eval {
        #[command(subcommand)]
        command: eval::Command,
    },
}

/// コマンドとサーバーが使うもの。AI は ANTHROPIC_API_KEY があるときだけ
pub struct Ctx {
    pub pool: SqlitePool,
    pub config: Config,
    pub llm: Option<Llm>,
    /// API キーがあれば AI で判定する（無ければ全件承認）
    pub moderation: Moderation,
    pub tagger: Option<Tagger>,
}

impl Ctx {
    pub fn new(pool: SqlitePool, config: Config) -> anyhow::Result<Self> {
        let llm = match &config.anthropic_api_key {
            Some(key) => Some(
                Llm::anthropic(
                    AnthropicClient::new(key.clone())?,
                    config.llm_daily_budget_usd,
                )
                .with_extraction_budget(config.llm_extraction_daily_budget_usd),
            ),
            None => None,
        };
        let moderation = match &llm {
            Some(llm) => Moderation::Llm {
                llm: llm.clone(),
                model: config.moderation_model.clone(),
            },
            None => Moderation::AutoApprove,
        };
        let tagger = llm.as_ref().map(|llm| Tagger {
            llm: llm.clone(),
            model: config.tagging_model.clone(),
        });
        Ok(Self {
            pool,
            config,
            llm,
            moderation,
            tagger,
        })
    }

    /// AI が要るコマンドで、API キーが無ければ止める
    fn llm(&self, what: &str) -> anyhow::Result<&Llm> {
        self.llm
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY が設定されていないため、{what}"))
    }

    /// 取得した原本の置き場
    fn store(&self) -> Store {
        Store::new(&self.config.private_dir)
    }
}

pub async fn run(ctx: &Ctx, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Fetch(args) => ingest::fetch(ctx, args).await,
        Command::Extract(args) => ingest::extract(ctx, args).await,
        Command::Import(args) => ingest::import(ctx, args).await,
        Command::List(args) => records::list(ctx, args).await,
        Command::Show(args) => records::show(ctx, args).await,
        Command::Edit(args) => records::edit(ctx, args).await,
        Command::Add(args) => records::add(ctx, args).await,
        Command::Remove(args) => records::remove(ctx, args).await,
        Command::Publish(args) => records::publish(ctx, args, true).await,
        Command::Archive(args) => records::publish(ctx, args, false).await,
        Command::Moderate(args) => process::moderate(ctx, args).await,
        Command::Classify(args) => process::classify(ctx, args).await,
        Command::Analyze(args) => process::analyze(ctx, args).await,
        Command::Seed { dev } => {
            let mut dirs = vec![ctx.config.seeds_dir.as_str()];
            if dev {
                dirs.push(ctx.config.dev_seeds_dir.as_str());
            }
            for dir in dirs {
                let files = seed::run(&ctx.pool, Path::new(dir)).await?;
                tracing::info!(dir, count = files.len(), "seeding finished");
            }
            Ok(())
        }
        Command::Eval { command } => eval::run(ctx, command).await,
    }
}

/// その対象には要らない処理。何もせずに通す（終了コード 0。手順を対象によらず同じに書けるように）
fn skip(verb: &str, target: impl clap::ValueEnum, reason: &str) -> anyhow::Result<()> {
    let target = target
        .to_possible_value()
        .map(|v| v.get_name().to_owned())
        .unwrap_or_default();
    println!("{verb} {target}: {reason}ので、何もしません");
    Ok(())
}

/// 対象に要るオプション。無ければ使い方を出して止める
fn required<'a, T>(value: &'a Option<T>, usage: &str) -> anyhow::Result<&'a T> {
    value
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("使い方: {usage}"))
}

/// 自治体の slug → 団体コード。運営のコマンドなので、公開前の自治体も選べる
async fn municipality_id(pool: &SqlitePool, slug: &str) -> anyhow::Result<String> {
    sqlx::query_scalar("SELECT id FROM municipalities WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("自治体が見つかりません: {slug}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_definitions_are_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn verbs_take_the_target_first() {
        let cli = Cli::try_parse_from(["backend", "show", "budget", "-m", "otawara", "-y", "2026"])
            .unwrap();
        assert!(matches!(cli.command, Some(Command::Show(_))));
        // 段階に無い対象は受け付けない
        assert!(Cli::try_parse_from(["backend", "fetch", "topic"]).is_err());
        assert!(Cli::try_parse_from(["backend", "analyze", "budget"]).is_err());
        // 位置で指す形は作らない
        assert!(Cli::try_parse_from(["backend", "show", "topic", "sample-1"]).is_err());
    }
}
