// SPDX-License-Identifier: AGPL-3.0-only

//! 書き込みの回数の制限（フェーズ7b）。同じ IP からの投票・投稿・提案・パスキーの手続き・関わり方の登録を、
//! 一定の時間の窓ごとに数える。匿名で投票できるので、機械的な大量の書き込みを少しでも抑える
//! （1人を見分ける本格的な対策は 6e）。
//!
//! スマホの回線は、多くの人が同じ IP を分け合う（キャリアの NAT）ので、上限は人の操作より十分に大きくする。
//! サーバー1台の前提で、数はメモリに持つ（再起動で消えてよい）。
//!
//! IP: 本番では Caddy が `X-Forwarded-For` に接続元を入れる（Caddy は信頼しない相手から来た
//! `X-Forwarded-*` を捨てる）ので、`TRUST_FORWARDED_FOR=true` のときだけその値を使う。
//! それ以外は接続元のアドレス（取れなければ1つの共通の枠）

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::AppState;
use super::error::ApiError;

/// 数える書き込みの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Vote,
    Statement,
    Proposal,
    Auth,
    Relation,
}

impl Kind {
    /// (上限, 窓の長さ)
    fn limit(self) -> (u32, Duration) {
        const MINUTE: Duration = Duration::from_secs(60);
        const HOUR: Duration = Duration::from_secs(3600);
        match self {
            // 1人が続けて投票しても1分に数十票。同じ IP を分け合う人数を見込んで大きめに
            Self::Vote => (300, MINUTE),
            Self::Statement => (30, MINUTE),
            // 提案は1人が1自治体に週1件（ADR 0005）
            Self::Proposal => (20, HOUR),
            Self::Auth => (60, MINUTE),
            Self::Relation => (60, MINUTE),
        }
    }

    /// API のパス（`/api` の下）とメソッドから種類を決める。運営者の API と読み出しは数えない
    pub fn of(method: &Method, path: &str) -> Option<Self> {
        if path.starts_with("/api/operator/") {
            return None;
        }
        match *method {
            Method::POST if path.starts_with("/api/auth/") => Some(Self::Auth),
            Method::DELETE if path == "/api/auth/me" => Some(Self::Auth),
            Method::POST if path.ends_with("/votes") => Some(Self::Vote),
            Method::POST if path.ends_with("/statements") => Some(Self::Statement),
            Method::POST if path.ends_with("/proposals") => Some(Self::Proposal),
            Method::PUT if path.ends_with("/relation") => Some(Self::Relation),
            _ => None,
        }
    }
}

struct Window {
    started: Instant,
    count: u32,
}

/// 数の表を掃除する目安（これを超えたら、窓の過ぎたものを捨てる）
const SWEEP_AT: usize = 10_000;

pub struct RateLimiter {
    windows: Mutex<HashMap<(Kind, String), Window>>,
    trust_forwarded_for: bool,
    enabled: bool,
}

impl RateLimiter {
    pub fn new(trust_forwarded_for: bool) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            trust_forwarded_for,
            enabled: true,
        }
    }

    /// 数えない（テストの多くは同じ接続元から続けて書き込むため）
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new(false)
        }
    }

    /// 1回数え、上限を超えたら false
    pub fn hit(&self, kind: Kind, client: &str, now: Instant) -> bool {
        if !self.enabled {
            return true;
        }
        let (max, window) = kind.limit();
        let mut windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if windows.len() >= SWEEP_AT {
            windows.retain(|(k, _), w| now.duration_since(w.started) < k.limit().1);
        }
        let w = windows.entry((kind, client.to_owned())).or_insert(Window {
            started: now,
            count: 0,
        });
        if now.duration_since(w.started) >= window {
            *w = Window {
                started: now,
                count: 0,
            };
        }
        w.count += 1;
        w.count <= max
    }

    /// 接続元（`X-Forwarded-For` は信頼する設定のときだけ。Caddy が付けた一番右の値）
    fn client(&self, req: &Request) -> String {
        if self.trust_forwarded_for
            && let Some(ip) = req
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit(',').next())
                .map(str::trim)
                .filter(|v| !v.is_empty())
        {
            return ip.to_owned();
        }
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    }
}

/// 中間処理: 数える書き込みなら数え、超えたら 429
pub async fn limit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    if let Some(kind) = Kind::of(req.method(), req.uri().path()) {
        let client = state.rate_limiter.client(&req);
        if !state.rate_limiter.hit(kind, &client, Instant::now()) {
            tracing::warn!(?kind, "rate limited");
            return ApiError::TooManyRequests.into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_decided_by_method_and_path() {
        let v = "/api/municipalities/otawara/topics/t1/statements/s1/votes";
        assert_eq!(Kind::of(&Method::POST, v), Some(Kind::Vote));
        assert_eq!(
            Kind::of(
                &Method::POST,
                "/api/municipalities/otawara/topics/t1/statements"
            ),
            Some(Kind::Statement)
        );
        assert_eq!(
            Kind::of(&Method::POST, "/api/auth/login/start"),
            Some(Kind::Auth)
        );
        assert_eq!(Kind::of(&Method::GET, "/api/auth/me"), None);
        assert_eq!(
            Kind::of(
                &Method::POST,
                "/api/operator/municipalities/otawara/statements/s1/approve"
            ),
            None
        );
    }

    #[test]
    fn the_limit_resets_after_the_window() {
        let limiter = RateLimiter::new(false);
        let start = Instant::now();
        let (max, window) = Kind::Statement.limit();
        for _ in 0..max {
            assert!(limiter.hit(Kind::Statement, "1.2.3.4", start));
        }
        assert!(!limiter.hit(Kind::Statement, "1.2.3.4", start));
        // ほかの人・ほかの種類は別に数える
        assert!(limiter.hit(Kind::Statement, "5.6.7.8", start));
        assert!(limiter.hit(Kind::Vote, "1.2.3.4", start));
        assert!(limiter.hit(Kind::Statement, "1.2.3.4", start + window));
    }
}
