use std::env;

const DEFAULT_DATABASE_URL: &str = "sqlite://../data/civic.db?mode=rwc";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";

pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
}

impl Config {
    /// 環境変数から読み込む。未設定の項目は開発用のデフォルト値を使う。
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into()),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into()),
        }
    }
}
