// SPDX-License-Identifier: AGPL-3.0-only

use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

/// `backend/migrations/` をバイナリに埋め込む。
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// SQLite に接続する。DB ファイルとその親ディレクトリが無ければ作成する。
pub async fn connect(database_url: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    if let Some(parent) = options.get_filename().parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    Ok(pool)
}
