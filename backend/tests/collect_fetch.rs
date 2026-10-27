// SPDX-License-Identifier: AGPL-3.0-only

//! 取得の共通処理（robots.txt・間隔・条件付き取得・大きさの上限）を、手元の HTTP サーバーで確かめる

mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Router;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use backend::collect::fetch::{self, FetchError, Fetched, Fetcher};

/// 受けたリクエストのパスと時刻
type Log = Arc<Mutex<Vec<(String, Instant)>>>;

async fn serve() -> (String, Log) {
    let log: Log = Arc::default();
    let record = |log: Log, path: &'static str| {
        move || {
            log.lock().unwrap().push((path.to_owned(), Instant::now()));
        }
    };
    let app = Router::new()
        .route(
            "/robots.txt",
            get({
                let r = record(log.clone(), "/robots.txt");
                move || async move {
                    r();
                    "User-agent: *\nDisallow: /private/\n"
                }
            }),
        )
        .route(
            "/file",
            get({
                let r = record(log.clone(), "/file");
                move |headers: HeaderMap| async move {
                    r();
                    if headers
                        .get(header::IF_NONE_MATCH)
                        .and_then(|v| v.to_str().ok())
                        == Some("\"v1\"")
                    {
                        return StatusCode::NOT_MODIFIED.into_response();
                    }
                    ([(header::ETAG, "\"v1\"")], "hello").into_response()
                }
            }),
        )
        .route(
            "/big",
            get({
                let r = record(log.clone(), "/big");
                move || async move {
                    r();
                    "x".repeat(2048)
                }
            }),
        )
        .route(
            "/private/secret",
            get({
                let r = record(log.clone(), "/private/secret");
                move || async move {
                    r();
                    "secret"
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), log)
}

async fn pool_with_source() -> sqlx::SqlitePool {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    sqlx::query(
        "INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url)
         VALUES ('src1', '092100', 'settlement_card', 'test', 'http://localhost/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

fn paths(log: &Log) -> Vec<String> {
    log.lock().unwrap().iter().map(|(p, _)| p.clone()).collect()
}

#[tokio::test]
async fn disallowed_paths_are_never_requested() {
    let (base, log) = serve().await;
    let fetcher = Fetcher::new("test@example.jp", Duration::ZERO).unwrap();
    let err = fetcher
        .get(&format!("{base}/private/secret"))
        .await
        .unwrap_err();
    assert!(matches!(err, FetchError::Disallowed(_)), "{err}");
    assert_eq!(paths(&log), ["/robots.txt"]);
}

#[tokio::test]
async fn robots_txt_is_cached_and_requests_are_spaced() {
    let (base, log) = serve().await;
    let interval = Duration::from_millis(300);
    let fetcher = Fetcher::new("test@example.jp", interval).unwrap();
    fetcher.get(&format!("{base}/big")).await.unwrap();
    fetcher.get(&format!("{base}/big")).await.unwrap();
    let log = log.lock().unwrap().clone();
    let p: Vec<&str> = log.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(p, ["/robots.txt", "/big", "/big"]);
    for pair in log.windows(2) {
        // 計測の誤差を見込んで少しだけ緩める
        assert!(
            pair[1].1 - pair[0].1 >= interval - Duration::from_millis(20),
            "間隔が短すぎます: {:?}",
            pair[1].1 - pair[0].1
        );
    }
}

#[tokio::test]
async fn conditional_get_uses_the_remembered_etag() {
    let (base, log) = serve().await;
    let pool = pool_with_source().await;
    let fetcher = Fetcher::new("test@example.jp", Duration::ZERO).unwrap();
    let url = format!("{base}/file");

    let Fetched::Body { bytes, cache } = fetcher.get_if_changed(&pool, "src1", &url).await.unwrap()
    else {
        panic!("初回は本文が返るはず");
    };
    assert_eq!(bytes, b"hello");
    assert_eq!(cache.etag.as_deref(), Some("\"v1\""));

    // 記録する前は、処理が終わっていないとみなして取り直す
    assert!(matches!(
        fetcher.get_if_changed(&pool, "src1", &url).await.unwrap(),
        Fetched::Body { .. }
    ));

    fetch::remember(&pool, &cache).await.unwrap();
    assert!(matches!(
        fetcher.get_if_changed(&pool, "src1", &url).await.unwrap(),
        Fetched::NotModified
    ));
    assert_eq!(paths(&log).iter().filter(|p| *p == "/file").count(), 3);
}

#[tokio::test]
async fn cache_is_per_source() {
    // 同じ県の Excel を別の自治体の収集先が使っても、それぞれ取り込まれる
    let (base, _log) = serve().await;
    let pool = pool_with_source().await;
    sqlx::query(
        "INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url)
         VALUES ('src2', '092134', 'settlement_card', 'test', 'http://localhost/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let fetcher = Fetcher::new("test@example.jp", Duration::ZERO).unwrap();
    let url = format!("{base}/file");
    let Fetched::Body { cache, .. } = fetcher.get_if_changed(&pool, "src1", &url).await.unwrap()
    else {
        panic!("初回は本文が返るはず");
    };
    fetch::remember(&pool, &cache).await.unwrap();
    assert!(matches!(
        fetcher.get_if_changed(&pool, "src2", &url).await.unwrap(),
        Fetched::Body { .. }
    ));
}

#[tokio::test]
async fn too_large_files_are_rejected() {
    let (base, _log) = serve().await;
    let fetcher = Fetcher::new("test@example.jp", Duration::ZERO)
        .unwrap()
        .with_max_bytes(1024);
    let err = fetcher.get(&format!("{base}/big")).await.unwrap_err();
    assert!(matches!(err, FetchError::TooLarge { .. }), "{err}");
}
