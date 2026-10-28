// SPDX-License-Identifier: AGPL-3.0-only

//! 取得の共通処理（`docs/design/collection.md`「取得の共通処理」）。
//! アダプタは HTTP を直接使わず、必ずここを通す。
//!
//! - robots.txt を確かめ、不許可の URL には接続しない（サイトごとに1日キャッシュ）
//! - 1サイトあたり `min_interval` 以上あける。並列には取得しない
//! - ETag / Last-Modified で条件付き取得する（`fetch_cache`）
//! - 大きさの上限を超えたら、取得をやめる

use std::collections::HashMap;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use reqwest::header::{CONTENT_LENGTH, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use super::robots::Robots;

/// User-Agent の製品名。robots.txt のグループもこの名前で探す
pub const PRODUCT: &str = "civic-platform-collector";
/// 1ファイルの大きさの上限
pub const DEFAULT_MAX_BYTES: usize = 100 * 1024 * 1024;
const ROBOTS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("robots.txt で許可されていません: {0}")]
    Disallowed(String),
    #[error("大きさの上限（{limit} バイト）を超えました: {url}")]
    TooLarge { url: String, limit: usize },
    #[error("HTTP {status}: {url}")]
    Status { url: String, status: u16 },
    #[error("取得に失敗しました: {0}")]
    Transport(String),
    #[error("URL が正しくありません: {0}")]
    InvalidUrl(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// 条件付き取得の記録（`fetch_cache` の1行）。取り込みまでの間は原本の記録（`store::Meta`）に入れておく
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub source_id: String,
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub sha256: String,
}

#[derive(Debug)]
pub enum Fetched {
    /// 前回から変わっていない（304、または中身のハッシュが同じ）
    NotModified,
    Body {
        bytes: Vec<u8>,
        cache: CacheEntry,
    },
}

pub struct Fetcher {
    http: reqwest::Client,
    min_interval: Duration,
    max_bytes: usize,
    /// サイト（scheme://host:port）ごとの robots.txt と取得した時刻
    robots: Mutex<HashMap<String, (Instant, Robots)>>,
    /// サイトごとの最後の接続の時刻。ロックを持ったまま待つので、並列にもならない
    last_request: Mutex<HashMap<String, Instant>>,
}

impl Fetcher {
    /// `contact` は運営者の連絡先（`COLLECTOR_CONTACT`）。User-Agent に入れる
    pub fn new(contact: &str, min_interval: Duration) -> anyhow::Result<Self> {
        // TLS の暗号処理に ring を使う（llm/client.rs と同じ）
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .user_agent(format!(
                "{PRODUCT}/{} (+{contact})",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(TIMEOUT)
            .build()?;
        Ok(Self {
            http,
            min_interval,
            max_bytes: DEFAULT_MAX_BYTES,
            robots: Mutex::new(HashMap::new()),
            last_request: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// 条件付きにせず取得する（一覧ページなど）
    pub async fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        match self.request(url, None).await? {
            Response::Body(bytes, _) => Ok(bytes),
            Response::NotModified => unreachable!("条件なしの取得で 304 は返らない"),
        }
    }

    /// `fetch_cache` の記録で条件付きに取得する。中身が変わっていなければ `NotModified`。
    /// 記録の更新は、呼び出し側が処理を終えてから `remember` で行う（途中で失敗したら、次も取り直すため）
    pub async fn get_if_changed(
        &self,
        pool: &SqlitePool,
        source_id: &str,
        url: &str,
    ) -> Result<Fetched, FetchError> {
        let previous = cached(pool, source_id, url).await?;
        match self.request(url, previous.as_ref()).await? {
            Response::NotModified => Ok(Fetched::NotModified),
            Response::Body(bytes, headers) => {
                let sha256 = hex(&Sha256::digest(&bytes));
                if previous.as_ref().is_some_and(|p| p.sha256 == sha256) {
                    return Ok(Fetched::NotModified);
                }
                let header = |name| {
                    headers
                        .get(name)
                        .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
                        .map(str::to_owned)
                };
                Ok(Fetched::Body {
                    cache: CacheEntry {
                        source_id: source_id.to_owned(),
                        url: url.to_owned(),
                        etag: header(ETAG),
                        last_modified: header(LAST_MODIFIED),
                        sha256,
                    },
                    bytes,
                })
            }
        }
    }

    async fn request(
        &self,
        url: &str,
        previous: Option<&CacheEntry>,
    ) -> Result<Response, FetchError> {
        let parsed = reqwest::Url::parse(url).map_err(|_| FetchError::InvalidUrl(url.into()))?;
        let site = site_of(&parsed).ok_or_else(|| FetchError::InvalidUrl(url.into()))?;
        let robots = self.robots_for(&site).await?;
        let path = match parsed.query() {
            Some(q) => format!("{}?{q}", parsed.path()),
            None => parsed.path().to_owned(),
        };
        if !robots.is_allowed(&path) {
            return Err(FetchError::Disallowed(url.into()));
        }

        let mut req = self.http.get(parsed);
        if let Some(p) = previous {
            if let Some(etag) = &p.etag {
                req = req.header(IF_NONE_MATCH, etag);
            }
            if let Some(lm) = &p.last_modified {
                req = req.header(IF_MODIFIED_SINCE, lm);
            }
        }
        let mut res = self.send_paced(&site, req).await?;
        let status = res.status();
        if status == StatusCode::NOT_MODIFIED && previous.is_some() {
            return Ok(Response::NotModified);
        }
        if !status.is_success() {
            return Err(FetchError::Status {
                url: url.into(),
                status: status.as_u16(),
            });
        }
        let too_large = || FetchError::TooLarge {
            url: url.into(),
            limit: self.max_bytes,
        };
        let declared = res
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok());
        if declared.is_some_and(|n| n > self.max_bytes) {
            return Err(too_large());
        }
        let headers = res.headers().clone();
        let mut bytes = Vec::new();
        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|e| FetchError::Transport(e.to_string()))?
        {
            if bytes.len() + chunk.len() > self.max_bytes {
                return Err(too_large());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(Response::Body(bytes, headers))
    }

    /// サイトの robots.txt（1日キャッシュ）。404 などで無ければ、すべて許可
    async fn robots_for(&self, site: &str) -> Result<Robots, FetchError> {
        let mut cache = self.robots.lock().await;
        if let Some((at, robots)) = cache.get(site)
            && at.elapsed() < ROBOTS_TTL
        {
            return Ok(robots.clone());
        }
        let url = format!("{site}/robots.txt");
        let res = self.send_paced(site, self.http.get(&url)).await?;
        let status = res.status();
        let robots = if status.is_success() {
            let text = res
                .text()
                .await
                .map_err(|e| FetchError::Transport(e.to_string()))?;
            Robots::parse(&text, PRODUCT)
        } else if status.is_client_error() {
            Robots::allow_all()
        } else {
            // サーバーの障害のときは、許可が分からないので取得しない（RFC 9309）
            return Err(FetchError::Status {
                url,
                status: status.as_u16(),
            });
        };
        cache.insert(site.to_owned(), (Instant::now(), robots.clone()));
        Ok(robots)
    }

    /// 前回の接続から `min_interval` 以上あけて送る
    async fn send_paced(
        &self,
        site: &str,
        req: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, FetchError> {
        let mut last = self.last_request.lock().await;
        if let Some(at) = last.get(site) {
            let wait = self.min_interval.saturating_sub(at.elapsed());
            if !wait.is_zero() {
                tokio::time::sleep(wait).await;
            }
        }
        let res = req.send().await;
        last.insert(site.to_owned(), Instant::now());
        res.map_err(|e| FetchError::Transport(e.without_url().to_string()))
    }
}

enum Response {
    NotModified,
    Body(Vec<u8>, reqwest::header::HeaderMap),
}

fn site_of(url: &reqwest::Url) -> Option<String> {
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 中身のハッシュ（`fetch_cache.sha256`・`documents.sha256` と同じ形）
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

async fn cached(
    pool: &SqlitePool,
    source_id: &str,
    url: &str,
) -> Result<Option<CacheEntry>, sqlx::Error> {
    let row: Option<(Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT etag, last_modified, sha256 FROM fetch_cache WHERE source_id = $1 AND url = $2",
    )
    .bind(source_id)
    .bind(url)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(etag, last_modified, sha256)| CacheEntry {
        source_id: source_id.to_owned(),
        url: url.to_owned(),
        etag,
        last_modified,
        sha256,
    }))
}

/// 処理を終えた取得を記録し、次からの条件付き取得に使う
pub async fn remember(pool: &SqlitePool, entry: &CacheEntry) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO fetch_cache (source_id, url, etag, last_modified, sha256, fetched_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (source_id, url) DO UPDATE SET
             etag = excluded.etag, last_modified = excluded.last_modified,
             sha256 = excluded.sha256, fetched_at = excluded.fetched_at",
    )
    .bind(&entry.source_id)
    .bind(&entry.url)
    .bind(&entry.etag)
    .bind(&entry.last_modified)
    .bind(&entry.sha256)
    .bind(crate::util::now_iso())
    .execute(pool)
    .await?;
    Ok(())
}
