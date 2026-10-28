// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::interface::api::app;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok_with_db() {
    let pool = common::test_pool().await;
    let res = app(common::test_state(pool))
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body, json!({ "status": "ok", "db": "ok" }));
}

#[tokio::test]
async fn health_returns_503_when_db_closed() {
    let pool = common::test_pool().await;
    pool.close().await;
    let res = app(common::test_state(pool))
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn unknown_api_route_returns_json_404() {
    let pool = common::test_pool().await;
    let res = app(common::test_state(pool))
        .oneshot(Request::get("/api/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn short_session_secret_is_rejected() {
    let pool = common::test_pool().await;
    assert!(backend::interface::api::AppState::new(pool, "too-short", false).is_err());
}
