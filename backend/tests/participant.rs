// SPDX-License-Identifier: AGPL-3.0-or-later

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use backend::AppState;
use backend::participant::{COOKIE_NAME, MaybeParticipant, Participant};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// エクストラクターだけを試すテスト用ルーター
fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/write",
            post(|p: Participant| async move {
                let id = p.user_id.clone();
                (p, id).into_response()
            }),
        )
        .route(
            "/read",
            get(|MaybeParticipant(id): MaybeParticipant| async move {
                id.unwrap_or_else(|| "none".to_owned())
            }),
        )
        .with_state(state)
}

struct Res {
    status: StatusCode,
    set_cookie: Option<String>,
    body: String,
}

async fn send(state: &AppState, method: &str, path: &str, cookie: Option<&str>) -> Res {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let res = router(state.clone())
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let set_cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned());
    let body =
        String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    Res {
        status,
        set_cookie,
        body,
    }
}

/// Set-Cookie から `name=value` 部分だけを取り出す
fn cookie_pair(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap()
}

async fn user_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn first_write_creates_user_and_sets_cookie() {
    let pool = common::test_pool().await;
    let state = common::test_state(pool.clone());

    let res = send(&state, "POST", "/write", None).await;
    assert_eq!(res.status, StatusCode::OK);
    let set_cookie = res.set_cookie.expect("新しい参加者には Set-Cookie を返す");
    assert!(set_cookie.starts_with(&format!("{COOKIE_NAME}=")));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/"));
    assert!(
        !set_cookie.contains("Secure"),
        "開発設定では Secure を付けない"
    );
    assert_eq!(user_count(&pool).await, 1);
}

#[tokio::test]
async fn valid_cookie_reuses_user() {
    let pool = common::test_pool().await;
    let state = common::test_state(pool.clone());

    let first = send(&state, "POST", "/write", None).await;
    let cookie = cookie_pair(first.set_cookie.as_deref().unwrap()).to_owned();

    let second = send(&state, "POST", "/write", Some(&cookie)).await;
    assert_eq!(second.body, first.body);
    assert!(second.set_cookie.is_none());
    assert_eq!(user_count(&pool).await, 1);

    let read = send(&state, "GET", "/read", Some(&cookie)).await;
    assert_eq!(read.body, first.body);
}

#[tokio::test]
async fn tampered_cookie_is_treated_as_absent() {
    let pool = common::test_pool().await;
    let state = common::test_state(pool.clone());

    let first = send(&state, "POST", "/write", None).await;
    // 署名部分を残したまま ID を別の値に差し替える
    let original = cookie_pair(first.set_cookie.as_deref().unwrap());
    let tampered = format!("{}x", original);

    let read = send(&state, "GET", "/read", Some(&tampered)).await;
    assert_eq!(read.body, "none");

    let write = send(&state, "POST", "/write", Some(&tampered)).await;
    assert_ne!(
        write.body, first.body,
        "改ざんされた Cookie では元のユーザーにならない"
    );
    assert!(write.set_cookie.is_some());
}

#[tokio::test]
async fn cookie_signed_with_other_key_is_rejected() {
    let pool = common::test_pool().await;
    let state = common::test_state(pool.clone());
    let other = AppState::new(
        pool.clone(),
        "another-secret-another-secret-0123456789",
        false,
    )
    .unwrap();

    let first = send(&other, "POST", "/write", None).await;
    let cookie = cookie_pair(first.set_cookie.as_deref().unwrap()).to_owned();

    let read = send(&state, "GET", "/read", Some(&cookie)).await;
    assert_eq!(read.body, "none");
}

#[tokio::test]
async fn read_does_not_create_user() {
    let pool = common::test_pool().await;
    let state = common::test_state(pool.clone());

    let res = send(&state, "GET", "/read", None).await;
    assert_eq!(res.body, "none");
    assert!(res.set_cookie.is_none());
    assert_eq!(user_count(&pool).await, 0);
}

#[tokio::test]
async fn secure_flag_follows_config() {
    let pool = common::test_pool().await;
    let state = AppState::new(pool, common::TEST_SECRET, true).unwrap();

    let res = send(&state, "POST", "/write", None).await;
    assert!(res.set_cookie.unwrap().contains("Secure"));
}
