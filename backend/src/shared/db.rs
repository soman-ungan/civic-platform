// SPDX-License-Identifier: AGPL-3.0-only

use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

/// `backend/migrations/` をバイナリに埋め込む。
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// SQLite に接続する。DB ファイルとその親ディレクトリが無ければ作成する。
pub async fn connect(database_url: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        // WAL では NORMAL で十分に安全（電源断で最後の書き込みを失うことはあっても、DB は壊れない）。
        // Litestream も WAL と NORMAL を前提にしている
        .synchronous(SqliteSynchronous::Normal)
        // ほかの書き込みが終わるのを待つ時間。超えると database is locked（ApiError がログに印を付ける）
        .busy_timeout(std::time::Duration::from_secs(5));

    if let Some(parent) = options.get_filename().parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    Ok(pool)
}
