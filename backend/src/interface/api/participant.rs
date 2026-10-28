// SPDX-License-Identifier: AGPL-3.0-only

//! 匿名参加者の識別。認証（フェーズ6）までは、署名付き Cookie に users.id を入れて識別する。

use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponseParts, ResponseParts};
use axum_extra::extract::cookie::{Cookie, SameSite, SignedCookieJar};

use crate::interface::api::AppState;
use crate::interface::api::error::ApiError;
use crate::shared::util::{new_id, now_iso};

pub const COOKIE_NAME: &str = "cp_uid";
const COOKIE_MAX_AGE_DAYS: i64 = 365;

/// 署名を検証し、実在するユーザーを指す Cookie ならその ID を返す。
async fn current_user_id(parts: &Parts, state: &AppState) -> Result<Option<String>, ApiError> {
    let jar = SignedCookieJar::from_headers(&parts.headers, state.cookie_key.clone());
    let Some(cookie) = jar.get(COOKIE_NAME) else {
        return Ok(None);
    };
    let user_id = sqlx::query_scalar("SELECT id FROM users WHERE id = $1")
        .bind(cookie.value())
        .fetch_optional(&state.pool)
        .await?;
    Ok(user_id)
}

/// 読み取り用。参加者が未登録なら `None`（ユーザーは作らない）。
pub struct MaybeParticipant(pub Option<String>);

impl FromRequestParts<AppState> for MaybeParticipant {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        Ok(Self(current_user_id(parts, state).await?))
    }
}

/// 書き込み用の参加者。
///
/// ハンドラは `MaybeParticipant` で受け取り、入力の検証が済んでから `Participant::ensure` で確定する
/// （不正なリクエストのたびに匿名ユーザーが作られないようにするため）。
/// 新しく作った場合はレスポンスに Set-Cookie を付ける必要があるので、
/// ハンドラは戻り値のタプルにこの値を含めること（`IntoResponseParts` を実装している）。
pub struct Participant {
    pub user_id: String,
    set_cookie: Option<SignedCookieJar>,
}

impl Participant {
    /// 既存の参加者ならそのまま使い、未登録なら匿名ユーザーを作る。
    pub async fn ensure(state: &AppState, existing: Option<String>) -> Result<Self, ApiError> {
        if let Some(user_id) = existing {
            return Ok(Self {
                user_id,
                set_cookie: None,
            });
        }

        let user_id = new_id();
        sqlx::query("INSERT INTO users (id, created_at) VALUES ($1, $2)")
            .bind(&user_id)
            .bind(now_iso())
            .execute(&state.pool)
            .await?;

        let cookie = Cookie::build((COOKIE_NAME, user_id.clone()))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .secure(state.cookie_secure)
            .max_age(cookie::time::Duration::days(COOKIE_MAX_AGE_DAYS));
        let jar = SignedCookieJar::new(state.cookie_key.clone()).add(cookie);

        Ok(Self {
            user_id,
            set_cookie: Some(jar),
        })
    }

    pub fn is_new(&self) -> bool {
        self.set_cookie.is_some()
    }
}

impl IntoResponseParts for Participant {
    type Error = Infallible;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        match self.set_cookie {
            Some(jar) => jar.into_response_parts(res),
            None => Ok(res),
        }
    }
}
