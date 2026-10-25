// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;

const DEFAULT_DATABASE_URL: &str = "sqlite://../data/civic.db?mode=rwc";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_SEEDS_DIR: &str = "../data/seeds";

pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub seeds_dir: String,
}

impl Config {
    /// 環境変数から読み込む。未設定の項目は開発用のデフォルト値を使う。
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into()),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into()),
            seeds_dir: env::var("SEEDS_DIR").unwrap_or_else(|_| DEFAULT_SEEDS_DIR.into()),
        }
    }
}
