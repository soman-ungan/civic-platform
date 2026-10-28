// SPDX-License-Identifier: AGPL-3.0-only

//! 運営のコマンドを、実際のバイナリで動かして確かめる（使い捨ての DB と置き場で）。
//! - その対象には要らない処理が、何もせずに終了コード 0 で通ること
//! - 予算の fetch → import で、取得の記録が documents に入り、置き場の原本が消えること

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use axum::Router;
use axum::routing::get;
use backend::db::MIGRATOR;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

struct Env {
    dir: PathBuf,
    pool: SqlitePool,
}

impl Env {
    async fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("civic-cli-{name}-{}", backend::util::new_id()));
        std::fs::create_dir_all(dir.join("private")).unwrap();
        let options = SqliteConnectOptions::from_str(&database_url(&dir))
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        common::seed_fixture(&pool).await;
        sqlx::raw_sql(include_str!("../../data/seeds/020_field_tags.sql"))
            .execute(&pool)
            .await
            .unwrap();
        Self { dir, pool }
    }

    fn private(&self) -> PathBuf {
        self.dir.join("private")
    }

    /// バイナリを動かし、終了コードと標準出力を返す。.env は読まない（作業ディレクトリを使い捨てにする）。
    /// API キーは空にして、実際の API を呼ばないようにする
    async fn run(&self, args: &[&str]) -> (i32, String) {
        let dir = self.dir.clone();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        tokio::task::spawn_blocking(move || {
            let out = Command::new(env!("CARGO_BIN_EXE_backend"))
                .args(&args)
                .current_dir(&dir)
                .env("DATABASE_URL", database_url(&dir))
                .env("PRIVATE_DIR", dir.join("private"))
                .env("SESSION_SECRET", common::TEST_SECRET)
                .env("COLLECTOR_CONTACT", "test@example.jp")
                .env("ANTHROPIC_API_KEY", "")
                .env("RUST_LOG", "warn")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            (out.status.code().unwrap_or(-1), format!("{stdout}{stderr}"))
        })
        .await
        .unwrap()
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn database_url(dir: &Path) -> String {
    format!("sqlite://{}?mode=rwc", dir.join("cli.db").display())
}

#[tokio::test(flavor = "multi_thread")]
async fn unneeded_steps_pass_through() {
    let env = Env::new("skip").await;
    for args in [
        &["fetch", "minutes"][..],
        &["extract", "settlement"],
        &["import", "settlement", "-m", "otawara"],
        &["import", "budget", "-m", "otawara", "-y", "2026"],
        &["publish", "settlement"],
        &["publish", "statement", "--id", "s1"],
        &["edit", "budget"],
        &["add", "source"],
        &["show", "tag"],
        &["moderate", "topic"],
        &["classify", "statement"],
        &["analyze", "statement"],
    ] {
        let (code, out) = env.run(args).await;
        assert_eq!(code, 0, "{args:?}: {out}");
        assert!(out.contains("何もしません"), "{args:?}: {out}");
    }
    // 何もせずに通しても、意見の状態は変わらない
    let status: String = sqlx::query_scalar("SELECT status FROM statements WHERE id = 's1'")
        .fetch_one(&env.pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_options_are_errors() {
    let env = Env::new("usage").await;
    for args in [
        &["show", "topic"][..],
        &["show", "budget", "-m", "otawara"],
        &["extract", "minutes", "-m", "otawara"],
        &["moderate", "statement", "--id", "s1"],
    ] {
        let (code, out) = env.run(args).await;
        assert_ne!(code, 0, "{args:?}: {out}");
        assert!(out.contains("使い方"), "{args:?}: {out}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn records_and_moderation_use_the_same_verbs() {
    let env = Env::new("records").await;
    let (code, out) = env.run(&["list", "statement"]).await;
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("s1") && out.contains("[pending]"), "{out}");

    let (code, out) = env
        .run(&["moderate", "statement", "--id", "s1", "--approve"])
        .await;
    assert_eq!(code, 0, "{out}");
    let (_, out) = env.run(&["list", "statement"]).await;
    assert!(out.contains("確認待ちの意見はありません"), "{out}");
    let (_, out) = env
        .run(&["list", "statement", "--status", "approved"])
        .await;
    assert!(out.contains("s1"), "{out}");

    // 意見の詳細は、投票の数を出し、投稿者は出さない
    sqlx::query(
        "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at)
         VALUES ('v1', 'u2', 's1', 1, '2026-10-26T00:00:00Z')",
    )
    .execute(&env.pool)
    .await
    .unwrap();
    let (code, out) = env.run(&["show", "statement", "--id", "s1"]).await;
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("賛成 1・反対 0・パス 0"), "{out}");
    assert!(out.contains("市民の投稿") && !out.contains("u1"), "{out}");

    // 地区のタグは、選んだ自治体のものだけ
    let (_, out) = env.run(&["list", "tag", "-m", "otawara"]).await;
    assert!(out.contains("[分野]"), "{out}");
}

const R8_PDF: &str = "../data/private/budget/R8yosangaiyou.pdf";
const R8_EXTRACTION: &str = "../data/private/budget/r8-extraction.json";

/// 市のサイトの代わり（一覧ページと PDF）
async fn serve_budget(pdf: Vec<u8>) -> String {
    let app = Router::new()
        .route("/robots.txt", get(|| async { "User-agent: *\nAllow: /\n" }))
        .route(
            "/docs/list.html",
            get(|| async {
                r#"<a href="R8yosangaiyou.pdf">当初予算の概要(令和8年度)[PDF：3.61MB]</a>"#
            }),
        )
        .route("/docs/R8yosangaiyou.pdf", get(move || async move { pdf }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

#[tokio::test(flavor = "multi_thread")]
async fn budget_fetch_then_import_records_the_source_and_discards_the_original() {
    // 実物の PDF と共同作業の出力が要る（data/private/ は git の管理外）
    let (Ok(pdf), Ok(extraction)) = (
        std::fs::read(R8_PDF),
        std::fs::read_to_string(R8_EXTRACTION),
    ) else {
        return;
    };
    let env = Env::new("budget").await;
    let base = serve_budget(pdf).await;
    sqlx::query(
        "INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url, config, is_enabled)
         VALUES ('test-budget', '092100', 'budget', 'html_link_list', $1, $2, TRUE)",
    )
    .bind(format!("{base}/docs/list.html"))
    .bind(r#"{"link_pattern": "当初予算の概要", "terms_url": "https://example.jp/terms", "terms_checked_at": "2026-10-27", "min_interval_secs": 0}"#)
    .execute(&env.pool)
    .await
    .unwrap();

    let (code, out) = env.run(&["fetch", "budget", "-m", "otawara"]).await;
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("取得しました"), "{out}");
    let original = env.private().join("fetched/otawara/budget/2026.pdf");
    assert!(original.exists());

    // AI の出力が無ければ止める（import は API を呼ばない）
    let (code, out) = env
        .run(&["import", "budget", "-m", "otawara", "-y", "2026"])
        .await;
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("extract budget"), "{out}");

    // 共同作業で作った出力を、原本の隣に置く
    std::fs::write(
        env.private()
            .join("fetched/otawara/budget/2026.extraction.json"),
        &extraction,
    )
    .unwrap();
    let (code, out) = env
        .run(&["import", "budget", "-m", "otawara", "-y", "2026"])
        .await;
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("置き場の原本を消しました"), "{out}");
    assert!(!original.exists());
    assert!(
        !env.private()
            .join("fetched/otawara/budget/2026.extraction.json")
            .exists()
    );

    let (url, source, method): (String, String, String) = sqlx::query_as(
        "SELECT url, collector_source_id, extraction FROM documents WHERE kind = 'budget'",
    )
    .fetch_one(&env.pool)
    .await
    .unwrap();
    assert_eq!(url, format!("{base}/docs/R8yosangaiyou.pdf"));
    assert_eq!(source, "test-budget");
    assert_eq!(method, "manual");
    let status: String = sqlx::query_scalar("SELECT status FROM budgets WHERE fiscal_year = 2026")
        .fetch_one(&env.pool)
        .await
        .unwrap();
    assert_eq!(status, "draft");

    // 取り込みが済んだので、次の fetch では取り直さない（置き場に何も置かれない）
    let (code, out) = env.run(&["fetch", "budget", "-m", "otawara"]).await;
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("変わっていません"), "{out}");
    assert!(!original.exists());

    // 手でコピーした原本は、取り込んでも消さない
    let manual = env.dir.join("R8.pdf");
    std::fs::copy(R8_PDF, &manual).unwrap();
    let manual_extraction = env.dir.join("r8.json");
    std::fs::write(&manual_extraction, &extraction).unwrap();
    let (code, out) = env
        .run(&[
            "import",
            "budget",
            "-m",
            "otawara",
            "-y",
            "2026",
            "--file",
            manual.to_str().unwrap(),
            "--extraction",
            manual_extraction.to_str().unwrap(),
            "--again",
        ])
        .await;
    assert_eq!(code, 0, "{out}");
    assert!(manual.exists());

    // 段階2の同じ動詞で、確かめて公開する
    let (code, out) = env
        .run(&["show", "budget", "-m", "otawara", "-y", "2026"])
        .await;
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("[draft]"), "{out}");
    let (code, _) = env
        .run(&["publish", "budget", "-m", "otawara", "-y", "2026"])
        .await;
    assert_eq!(code, 0);
    let (_, out) = env.run(&["list", "budget", "-m", "otawara"]).await;
    assert!(out.contains("[published]"), "{out}");
}
