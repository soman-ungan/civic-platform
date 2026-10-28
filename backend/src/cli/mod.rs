// SPDX-License-Identifier: AGPL-3.0-only

//! 運営のコマンド（`cargo run -- <コマンド>`）。使い方の全体は `docs/cli.md`、各コマンドは `--help`。
//! コマンドの処理はここに置き、中身（取り込み・判定・公開など）はライブラリのモジュールに任せる

mod budget;
mod collect;
mod eval;
mod import;
mod moderation;
mod topics;

use std::path::Path;

use backend::llm::Llm;
use backend::llm::client::AnthropicClient;
use backend::moderation::Moderation;
use backend::tagging::{self, Tagger};
use backend::{analysis, config::Config, seed};
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
    /// マスタ（自治体・地区・分野タグ・公人の名簿・収集先）を入れる
    Seed {
        /// 続けて data/dev-seeds/（【サンプル】の架空のデータ）も入れる。本番では使わない
        #[arg(long)]
        dev: bool,
    },
    /// 意見の分析（意見マップ・グループ・合意点）を、すぐに計算し直す
    Analyze {
        /// 1つのトピックだけ
        #[arg(long)]
        topic: Option<String>,
        /// 投票が変わっていなくても計算する
        #[arg(long)]
        force: bool,
    },
    /// 確認待ちの意見を見る・公開する・非公開にする（既定は list）
    Moderation {
        #[command(subcommand)]
        command: Option<moderation::Command>,
    },
    /// トピックに分野・地区のタグを AI で付ける（ANTHROPIC_API_KEY が要る）
    Tag {
        /// 1つのトピックだけ（無ければ、タグの無いトピックすべて）
        #[arg(long)]
        topic: Option<String>,
        /// AI が付けたタグだけを付け直す（運営者が付けたタグは残す）
        #[arg(long)]
        force: bool,
    },
    /// 会議録・予算の概要を取り込んで、下書きを作る（費用がかかる）
    Import {
        #[command(subcommand)]
        command: import::Command,
    },
    /// トピックの下書きを確かめ、手直しし、公開する（既定は drafts）
    Topics {
        #[command(subcommand)]
        command: Option<topics::Command>,
    },
    /// 行政資料（決算カード・予算の概要）を取得する（既定は list）
    Collect {
        #[command(subcommand)]
        command: Option<collect::Command>,
    },
    /// 予算の下書きを確かめ、公開する（既定は list）
    Budget {
        #[command(subcommand)]
        command: Option<budget::Command>,
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
}

pub async fn run(ctx: &Ctx, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Seed { dev } => {
            let mut dirs = vec![ctx.config.seeds_dir.as_str()];
            if dev {
                dirs.push(ctx.config.dev_seeds_dir.as_str());
            }
            for dir in dirs {
                let files = seed::run(&ctx.pool, Path::new(dir)).await?;
                tracing::info!(dir, count = files.len(), "seeding finished");
            }
        }
        Command::Analyze { topic, force } => match topic {
            Some(topic_id) => {
                let outcome = analysis::store::run_topic(&ctx.pool, &topic_id, force).await?;
                tracing::info!(topic_id, ?outcome, "analysis finished");
            }
            None => {
                let n = analysis::store::run_all(&ctx.pool, force).await?;
                tracing::info!(topics = n, "analysis finished");
            }
        },
        Command::Moderation { command } => moderation::run(ctx, command).await?,
        Command::Tag { topic, force } => {
            let Some(tagger) = &ctx.tagger else {
                anyhow::bail!("ANTHROPIC_API_KEY が設定されていないため、タグ付けできません");
            };
            match topic {
                Some(topic_id) => {
                    let outcome = tagging::tag_topic(&ctx.pool, tagger, &topic_id, force).await?;
                    println!("{topic_id}: {outcome:?}");
                }
                None => {
                    let n = tagging::run_untagged(&ctx.pool, tagger).await?;
                    println!("{n} 件のトピックをタグ付けしました");
                }
            }
        }
        Command::Import { command } => import::run(ctx, command).await?,
        Command::Topics { command } => topics::run(ctx, command).await?,
        Command::Collect { command } => collect::run(ctx, command).await?,
        Command::Budget { command } => budget::run(ctx, command).await?,
        Command::Eval { command } => eval::run(ctx, command).await?,
    }
    Ok(())
}

/// 自治体の slug → 団体コード。運営のコマンドなので、公開前の自治体も選べる
async fn municipality_id(pool: &SqlitePool, slug: &str) -> anyhow::Result<String> {
    sqlx::query_scalar("SELECT id FROM municipalities WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("自治体が見つかりません: {slug}"))
}
