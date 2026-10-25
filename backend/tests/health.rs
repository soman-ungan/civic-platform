mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backend::{AppState, app};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok_with_db() {
    let pool = common::test_pool().await;
    let res = app(AppState { pool })
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
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
    let res = app(AppState { pool })
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
