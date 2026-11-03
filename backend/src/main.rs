// SPDX-License-Identifier: AGPL-3.0-only

use backend::interface::api::rate_limit::RateLimiter;
use backend::interface::api::{AppState, app};
use backend::interface::cli;
use backend::process::moderation::{self, Moderation};
use backend::process::{analysis, publishing, tagging};
use backend::shared::{config::Config, db};
use clap::Parser;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --help などは、設定や DB が無くても出す
    let args = cli::Cli::parse();

    // ルートの .env を親ディレクトリまで遡って読む（無くてもよい）
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    db::MIGRATOR.run(&pool).await?;
    tracing::info!("migrations applied");

    let ctx = cli::Ctx::new(pool, config)?;
    match args.command {
        Some(command) => cli::run(&ctx, command).await,
        None => serve(ctx).await,
    }
}

/// API サーバーと、バックグラウンドの処理（分析・自動公開・判定のやり直し・タグ付け）
async fn serve(ctx: cli::Ctx) -> anyhow::Result<()> {
    let cli::Ctx {
        pool,
        config,
        moderation,
        tagger,
        ..
    } = ctx;
    config.check_for_serving()?;

    if config.analysis_interval_secs > 0 {
        analysis::store::spawn_worker(
            pool.clone(),
            std::time::Duration::from_secs(config.analysis_interval_secs),
        );
    }

    publishing::spawn_worker(pool.clone());

    match &moderation {
        Moderation::Llm { model, .. } => {
            tracing::info!(model, "AI moderation enabled");
            moderation::spawn_retry_worker(
                pool.clone(),
                moderation.clone(),
                std::time::Duration::from_secs(60),
            );
        }
        Moderation::AutoApprove => {
            tracing::info!("ANTHROPIC_API_KEY が無いので、投稿は自動で承認し、タグ付けはしません");
        }
    }
    if let Some(tagger) = tagger {
        tagging::spawn_worker(pool.clone(), tagger, std::time::Duration::from_secs(60));
    }

    let state = AppState::new(pool, &config.session_secret, config.cookie_secure)?
        .with_moderation(moderation)
        .with_webauthn(&config.webauthn_rp_id, &config.webauthn_origin)?
        .with_rate_limiter(RateLimiter::new(config.trust_forwarded_for));
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

    // 接続元のアドレス（回数の制限で、X-Forwarded-For を使わないとき）
    axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("shutting down");
}
