use std::path::{Path, PathBuf};

use anyhow::Context;
use sqlx::SqlitePool;

/// `dir` 直下の `*.sql` をファイル名順に 1 トランザクションで流す。
/// seed ファイルは `ON CONFLICT DO NOTHING` で書かれている前提で、何度実行してもよい。
/// 適用したファイルのパスを返す。
pub async fn run(pool: &SqlitePool, dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("seed ディレクトリを読めません: {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    files.sort();

    let mut tx = pool.begin().await?;
    for path in &files {
        let sql = std::fs::read_to_string(path)
            .with_context(|| format!("seed ファイルを読めません: {}", path.display()))?;
        sqlx::raw_sql(&sql)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("seed の適用に失敗しました: {}", path.display()))?;
        tracing::info!(file = %path.display(), "seed applied");
    }
    tx.commit().await?;

    Ok(files)
}
