// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;

use backend::AppState;
use backend::db::MIGRATOR;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

/// マイグレーション済みのインメモリ DB を用意する。
/// インメモリ DB は接続ごとに別物になるので、接続数を 1 に固定する。
pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    pool
}

/// テスト用の署名鍵（32 バイト以上）
pub const TEST_SECRET: &str = "test-secret-test-secret-test-secret-0123456789";

#[allow(dead_code)] // テストクレートごとに使う関数が異なるため
pub fn test_state(pool: SqlitePool) -> AppState {
    AppState::new(pool, TEST_SECRET, false).unwrap()
}

/// テスト用の最小データ。自治体の取り違えを検出できるよう 2 自治体を入れる。
///
/// - 092100 大田原市（地区 092100-kurobane, トピック t1）
/// - 092134 那須塩原市（地区 092134-shiobara, トピック t2）
/// - ユーザー u1, u2 / ステートメント s1（t1 内）
#[allow(dead_code)] // テストクレートごとに使う関数が異なるため
pub async fn seed_fixture(pool: &SqlitePool) {
    sqlx::raw_sql(
        "
        INSERT INTO prefectures (code, name) VALUES ('09', '栃木県');
        INSERT INTO municipalities (id, prefecture_code, name, name_kana, slug, is_active, created_at) VALUES
            ('092100', '09', '大田原市', 'おおたわらし', 'otawara', TRUE, '2026-10-26T00:00:00Z'),
            ('092134', '09', '那須塩原市', 'なすしおばらし', 'nasushiobara', FALSE, '2026-10-26T00:00:00Z');
        INSERT INTO districts (id, municipality_id, name, name_kana) VALUES
            ('092100-kurobane', '092100', '黒羽', 'くろばね'),
            ('092134-shiobara', '092134', '塩原', 'しおばら');
        INSERT INTO users (id, created_at) VALUES
            ('u1', '2026-10-26T00:00:00Z'),
            ('u2', '2026-10-26T00:00:00Z');
        INSERT INTO topics (id, municipality_id, title, source_type, created_at) VALUES
            ('t1', '092100', '大田原のテスト', 'citizen', '2026-10-26T00:00:00Z'),
            ('t2', '092134', '那須塩原のテスト', 'citizen', '2026-10-26T00:00:00Z');
        INSERT INTO statements (id, topic_id, user_id, body, created_at) VALUES
            ('s1', 't1', 'u1', '意見', '2026-10-26T00:00:00Z');
        ",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// 那須塩原市も公開状態にする（自治体の取り違えを試すテスト用）
#[allow(dead_code)]
pub async fn activate_second_municipality(pool: &SqlitePool) {
    sqlx::query("UPDATE municipalities SET is_active = TRUE WHERE id = '092134'")
        .execute(pool)
        .await
        .unwrap();
}

#[allow(dead_code)]
pub struct TestResponse {
    pub status: axum::http::StatusCode,
    pub json: serde_json::Value,
    /// Set-Cookie の `name=value` 部分
    pub cookie: Option<String>,
}

/// アプリにリクエストを1つ送る。`body` があれば JSON として送る。
#[allow(dead_code)]
pub async fn request(
    state: &AppState,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> TestResponse {
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut req = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(json) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();

    let res = backend::app(state.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_owned());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            panic!(
                "JSON ではないレスポンス: {}",
                String::from_utf8_lossy(&bytes)
            )
        })
    };
    TestResponse {
        status,
        json,
        cookie,
    }
}

/// JSON のどの階層にも `key` が無いことを確かめる（投稿者情報の漏れ防止）
#[allow(dead_code)]
pub fn assert_no_key(value: &serde_json::Value, key: &str) {
    match value {
        serde_json::Value::Object(map) => {
            assert!(
                !map.contains_key(key),
                "レスポンスに {key} が含まれている: {value}"
            );
            map.values().for_each(|v| assert_no_key(v, key));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| assert_no_key(v, key)),
        _ => {}
    }
}
