// SPDX-License-Identifier: AGPL-3.0-only

//! 決算カードの取得と取り込みの流れ全体（一覧 → 年度のページ → Excel → 置き場 → 保存 → 原本を消す）。
//! 実物のページと Excel（data/private/soumu/）を手元のサーバーから返して確かめる

use axum::Router;
use axum::http::header;
use axum::routing::get;
use backend::ingest::collect::store::Store;
use backend::ingest::collect::{self, Options, settlement_card};

fn private(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("../data/private/soumu/{name}")).ok()
}

/// 総務省のサイトと同じパスで、実物のファイルを返す
async fn serve(index: Vec<u8>, year: Vec<u8>, excel: Vec<u8>) -> String {
    let app = Router::new()
        .route(
            "/robots.txt",
            get(|| async { "User-agent: ia_archiver\nDisallow: /\n" }),
        )
        .route("/iken/zaisei/card.html", get(move || async move { index }))
        .route(
            "/iken/zaisei/card-24.html",
            get(move || async move { year }),
        )
        .route(
            "/main_content/000998938.xlsx",
            get(move || async move { ([(header::ETAG, "\"r5\"")], excel) }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

async fn add_source(pool: &sqlx::SqlitePool, id: &str, municipality_id: &str, base: &str) {
    sqlx::query(
        "INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url, config, is_enabled)
         VALUES ($1, $2, 'settlement_card', 'soumu_settlement_card', $3, $4, TRUE)",
    )
    .bind(id)
    .bind(municipality_id)
    .bind(format!("{base}/iken/zaisei/card.html"))
    .bind(r#"{"terms_url": "https://example.jp/terms", "terms_checked_at": "2026-10-27", "min_interval_secs": 0}"#)
    .execute(pool)
    .await
    .unwrap();
}

/// テストごとの置き場
fn temp_store(name: &str) -> (std::path::PathBuf, Store) {
    let dir = std::env::temp_dir().join(format!(
        "civic-collect-{name}-{}",
        backend::shared::util::new_id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::new(&dir);
    (dir, store)
}

async fn fetch(
    pool: &sqlx::SqlitePool,
    id: &str,
    options: &Options,
    store: &Store,
) -> collect::Report {
    collect::fetch_source(pool, &source(pool, id).await, "test", options, store)
        .await
        .unwrap()
}

async fn source(pool: &sqlx::SqlitePool, id: &str) -> collect::Source {
    collect::sources(pool)
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap()
}

#[tokio::test]
#[ignore = "data/private/soumu/ に実物のページと Excel が要る"]
async fn collects_real_card_into_db() {
    let (Some(index), Some(year), Some(excel)) = (
        private("card.html"),
        private("card-24.html"),
        private("tochigi-r5.xlsx"),
    ) else {
        return;
    };
    let base = serve(index, year, excel).await;
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    add_source(&pool, "otawara", "092100", &base).await;
    add_source(&pool, "nasushiobara", "092134", &base).await;
    let options = Options {
        dry_run: false,
        fiscal_year: Some(2023),
    };

    let (dir, store) = temp_store("real");

    let report = fetch(&pool, "otawara", &options, &store).await;
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(
        report.lines[0].contains("取得しました"),
        "{:?}",
        report.lines
    );
    // 取得しただけでは、DB に入らない
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlement_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(store.originals("otawara", "settlement").unwrap().len(), 1);

    let report = settlement_card::import(&pool, &store, "otawara", None, false, false)
        .await
        .unwrap();
    assert!(
        report.lines[0].contains("保存しました"),
        "{:?}",
        report.lines
    );
    // 取り込みが済んだ原本は消える
    assert!(store.originals("otawara", "settlement").unwrap().is_empty());

    let total: i64 = sqlx::query_scalar(
        "SELECT SUM(value) FROM settlement_items
         WHERE municipality_id = '092100' AND fiscal_year = 2023 AND category = 'expenditure_purpose'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(total, 33694873);
    let population: i64 = sqlx::query_scalar(
        "SELECT population FROM municipality_stats WHERE municipality_id = '092100' AND fiscal_year = 2023",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(population, 68873);

    // 2回目は変化なし。置き場には何も置かれない
    let report = fetch(&pool, "otawara", &options, &store).await;
    assert!(
        report.lines[0].contains("変わっていません"),
        "{:?}",
        report.lines
    );
    assert!(store.originals("otawara", "settlement").unwrap().is_empty());

    // 同じ県の Excel でも、別の自治体の収集先はそれぞれ取り込み、取り違えない
    fetch(&pool, "nasushiobara", &options, &store).await;
    settlement_card::import(&pool, &store, "nasushiobara", None, false, false)
        .await
        .unwrap();
    let names: Vec<(String, i64)> = sqlx::query_as(
        "SELECT municipality_id, COUNT(*) FROM settlement_items GROUP BY municipality_id ORDER BY municipality_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(names.len(), 2);
    let other_total: i64 = sqlx::query_scalar(
        "SELECT SUM(value) FROM settlement_items
         WHERE municipality_id = '092134' AND fiscal_year = 2023 AND category = 'expenditure_purpose'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_ne!(other_total, total);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn sources_without_terms_record_do_not_run() {
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    sqlx::query(
        "INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url, config, is_enabled)
         VALUES ('s', '092100', 'settlement_card', 'soumu_settlement_card', 'http://127.0.0.1:9/', '{}', TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (dir, store) = temp_store("terms");
    let err = collect::fetch_source(
        &pool,
        &source(&pool, "s").await,
        "test",
        &Options::default(),
        &store,
    )
    .await
    .unwrap_err();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(err.to_string().contains("利用規約"), "{err}");
}

#[tokio::test]
async fn edit_source_changes_only_from_fiscal_year() {
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    sqlx::query(
        "INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url, config, is_enabled)
         VALUES ('b', '092100', 'budget', 'html_link_list', 'http://127.0.0.1:9/',
                 '{\"link_pattern\": \"^当初予算の概要\", \"from_fiscal_year\": 2026, \"min_interval_secs\": 5}', TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(
        collect::set_from_fiscal_year(&pool, "b", 2006)
            .await
            .unwrap()
    );
    let config = source(&pool, "b").await.config_json().unwrap();
    assert_eq!(config["from_fiscal_year"], 2006);
    // ほかの項目はそのまま
    assert_eq!(config["link_pattern"], "^当初予算の概要");
    assert_eq!(config["min_interval_secs"], 5);

    // 無い収集先
    assert!(
        !collect::set_from_fiscal_year(&pool, "none", 2006)
            .await
            .unwrap()
    );
}
