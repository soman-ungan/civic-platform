// SPDX-License-Identifier: AGPL-3.0-only

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, OptionalFromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// API のエラー。レスポンスは `{"error": {"code": "...", "message": "..."}}` 形式。
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("見つかりません")]
    NotFound,
    /// 入力値の検証エラー（422）
    #[error("{message}")]
    Validation { code: &'static str, message: String },
    /// ログインが要る（401）。例: パスキーを登録しないまま投稿した
    #[error("{message}")]
    Unauthorized { code: &'static str, message: String },
    /// まだできない（403）。例: 分析を見られるまで投票しないまま投稿した
    #[error("{message}")]
    Forbidden { code: &'static str, message: String },
    /// 状態の衝突（409）。例: 居住地属性が未登録のまま投票した
    #[error("{message}")]
    Conflict { code: &'static str, message: String },
    /// JSON として読めない・Content-Type が違うなど、リクエストの形式の誤り
    #[error("{message}")]
    InvalidRequest { status: StatusCode, message: String },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    /// 保存データの読み込みなど、その他の内部エラー
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        Self::Validation {
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self::Unauthorized {
            code,
            message: message.into(),
        }
    }

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self::Forbidden {
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            Self::Validation { code, message } => (StatusCode::UNPROCESSABLE_ENTITY, code, message),
            Self::Unauthorized { code, message } => (StatusCode::UNAUTHORIZED, code, message),
            Self::Forbidden { code, message } => (StatusCode::FORBIDDEN, code, message),
            Self::Conflict { code, message } => (StatusCode::CONFLICT, code, message),
            Self::InvalidRequest { status, message } => (status, "invalid_request", message),
            Self::Database(err) => {
                // 詳細はログにだけ出し、クライアントには返さない
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "サーバーでエラーが発生しました".to_owned(),
                )
            }
            Self::Internal(err) => {
                tracing::error!(error = %err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "サーバーでエラーが発生しました".to_owned(),
                )
            }
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}

/// `axum::Json` と同じだが、読み取りに失敗したときも API 共通の JSON エラーを返す。
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(rejection_error(rejection)),
        }
    }
}

/// 本文を省ける JSON（`Option<ApiJson<T>>`）。Content-Type が JSON でなければ None
impl<S, T> OptionalFromRequest<S> for ApiJson<T>
where
    Json<T>: OptionalFromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Option<Self>, Self::Rejection> {
        match <Json<T> as OptionalFromRequest<S>>::from_request(req, state).await {
            Ok(value) => Ok(value.map(|Json(v)| Self(v))),
            Err(rejection) => Err(rejection_error(rejection)),
        }
    }
}

fn rejection_error(rejection: JsonRejection) -> ApiError {
    ApiError::InvalidRequest {
        status: rejection.status(),
        message: rejection.body_text(),
    }
}
