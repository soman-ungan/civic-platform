// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use backend::{AppState, app, config::Config, db, seed};
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
    if std::env::args().nth(1).as_deref() == Some("seed") {
        let files = seed::run(&pool, Path::new(&config.seeds_dir)).await?;
        tracing::info!(count = files.len(), "seeding finished");
        return Ok(());
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
