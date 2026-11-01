// SPDX-License-Identifier: AGPL-3.0-only

//! 書き込みの回数の制限（フェーズ7b）: 上限を超えると 429、読み出しは数えない

use crate::common::{request, seed_fixture, test_pool, test_state};
use axum::http::StatusCode;
use backend::interface::api::rate_limit::RateLimiter;

#[tokio::test]
async fn too_many_posts_get_429_but_reads_are_not_limited() {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    let state = test_state(pool).with_rate_limiter(RateLimiter::new(false));
    let path = "/api/municipalities/otawara/topics/t1/statements";
    // 投稿の上限（1分に30回）までは、ふだんの応答（未登録なら 401 など）
    for _ in 0..30 {
        let res = request(&state, "POST", path, None, None).await;
        assert_ne!(res.status, StatusCode::TOO_MANY_REQUESTS);
    }
    let res = request(&state, "POST", path, None, None).await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(res.json["error"]["code"], "rate_limited");
    // 読み出しは数えない
    for _ in 0..50 {
        let res = request(
            &state,
            "GET",
            "/api/municipalities/otawara/topics",
            None,
            None,
        )
        .await;
        assert_eq!(res.status, StatusCode::OK);
    }
}
