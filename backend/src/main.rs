// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;

use backend::{AppState, analysis, app, config::Config, db, seed};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ルートの .env を親ディレクトリまで遡って読む（無くてもよい）
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    db::MIGRATOR.run(&pool).await?;
    tracing::info!("migrations applied");

    // `cargo run -- seed`: マスタ seed を投入して終了する
    // `cargo run -- seed --dev`: 続けて開発用サンプルデータも投入する（本番では使わない）
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("seed") {
        let mut dirs = vec![config.seeds_dir.as_str()];
        if args.iter().any(|a| a == "--dev") {
            dirs.push(config.dev_seeds_dir.as_str());
        }
        for dir in dirs {
            let files = seed::run(&pool, Path::new(dir)).await?;
            tracing::info!(dir, count = files.len(), "seeding finished");
        }
        return Ok(());
    }

    // `cargo run -- analyze [--topic <id>] [--force]`: 分析を更新して終了する（開発用）
    if args.first().map(String::as_str) == Some("analyze") {
        let force = args.iter().any(|a| a == "--force");
        let topic = args
            .iter()
            .position(|a| a == "--topic")
            .and_then(|i| args.get(i + 1));
        match topic {
            Some(topic_id) => {
                let outcome = analysis::store::run_topic(&pool, topic_id, force).await?;
                tracing::info!(topic_id, ?outcome, "analysis finished");
            }
            None => {
                let n = analysis::store::run_all(&pool, force).await?;
                tracing::info!(topics = n, "analysis finished");
            }
        }
        return Ok(());
    }

    if config.analysis_interval_secs > 0 {
        analysis::store::spawn_worker(
            pool.clone(),
            std::time::Duration::from_secs(config.analysis_interval_secs),
        );
    }

    let state = AppState::new(pool, &config.session_secret, config.cookie_secure)?;
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("shutting down");
}
