// SPDX-License-Identifier: AGPL-3.0-only

//! パスキー（WebAuthn）での登録とログイン（ADR 0006）。
//!
//! - 登録: 今の参加者（匿名なら作る）にパスキーを付ける。それまでの投票はそのまま
//! - ログイン: 名前を入力しないパスキー（discoverable credential）で、その利用者に切り替える。
//!   その端末の匿名の参加者の投票などは、アカウントに移す（`records::accounts::merge_anonymous`）
//! - 利用者の ID は返さない。返すのは登録済みか・運営者かだけ
//!
//! 途中の状態（challenge）は DB に置き、1回だけ使える。パスキーの表示名には個人の情報を入れない

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use webauthn_rs::prelude::{
    CreationChallengeResponse, DiscoverableAuthentication, DiscoverableKey, Passkey,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url, Uuid, Webauthn, WebauthnBuilder,
};

use super::AppState;
use super::error::{ApiError, ApiJson};
use super::participant::{self, MaybeParticipant, Participant};
use crate::records::accounts;

/// パスキーに付ける表示名（端末のパスキーの一覧に出る）。個人の情報も自治体名も入れない
const PASSKEY_NAME: &str = "civic-platform の参加者";

/// WebAuthn の設定を作る。`rp_id` はドメイン（開発では localhost）、`origin` は画面のオリジン
pub fn webauthn(rp_id: &str, origin: &str) -> anyhow::Result<Arc<Webauthn>> {
    let origin = Url::parse(origin)?;
    let webauthn = WebauthnBuilder::new(rp_id, &origin)?
        .rp_name("civic-platform")
        .build()?;
    Ok(Arc::new(webauthn))
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Me {
    pub registered: bool,
    pub operator: bool,
}

impl From<accounts::Status> for Me {
    fn from(s: accounts::Status) -> Self {
        Self {
            registered: s.registered,
            operator: s.operator,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Started<T> {
    pub challenge_id: String,
    pub options: T,
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinish {
    pub challenge_id: String,
    pub credential: RegisterPublicKeyCredential,
    /// 運営者の招待のトークン（CLI の add operator が出すリンク）
    #[serde(default)]
    pub operator_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginFinish {
    pub challenge_id: String,
    pub credential: PublicKeyCredential,
}

fn invalid_challenge() -> ApiError {
    ApiError::validation(
        "challenge_invalid",
        "登録・ログインの手続きの期限が切れました。もう一度やり直してください",
    )
}

fn invalid_passkey() -> ApiError {
    ApiError::validation(
        "passkey_invalid",
        "パスキーを確かめられませんでした。もう一度やり直してください",
    )
}

fn credential_id(passkey: &Passkey) -> Result<String, ApiError> {
    serde_json::to_value(passkey.cred_id())
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("credential ID を文字にできません").into())
}

fn json<T: Serialize>(value: &T) -> Result<String, ApiError> {
    serde_json::to_string(value).map_err(|e| anyhow::Error::from(e).into())
}

fn parse<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, ApiError> {
    serde_json::from_str(text).map_err(|e| anyhow::Error::from(e).into())
}

/// 名前を入力せずにほかの端末からログインできるよう、端末の側に持つ形のパスキー（discoverable credential）を求める。
/// webauthn-rs の `start_passkey_registration` は求めないので、ブラウザに渡す指示だけを書き換える
fn require_resident_key(options: CreationChallengeResponse) -> Result<Value, ApiError> {
    let mut value = serde_json::to_value(options).map_err(anyhow::Error::from)?;
    if let Some(selection) = value
        .pointer_mut("/publicKey/authenticatorSelection")
        .and_then(Value::as_object_mut)
    {
        selection.insert("residentKey".into(), "required".into());
        selection.insert("requireResidentKey".into(), true.into());
    }
    Ok(value)
}

/// GET /api/auth/me
pub async fn me(
    State(state): State<AppState>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<Me>, ApiError> {
    let status = match user_id {
        Some(id) => accounts::status(&state.pool, &id).await?,
        None => accounts::Status::default(),
    };
    Ok(Json(status.into()))
}

/// POST /api/auth/register/start — 今の参加者（匿名なら作る）にパスキーを付ける手続きを始める
pub async fn register_start(
    State(state): State<AppState>,
    MaybeParticipant(existing): MaybeParticipant,
) -> Result<(Participant, Json<Started<Value>>), ApiError> {
    let participant = Participant::ensure(&state, existing).await?;
    let user_uuid = Uuid::parse_str(&participant.user_id)
        .map_err(|e| anyhow::anyhow!("利用者の ID が UUID ではありません: {e}"))?;
    // 同じ端末に同じアカウントのパスキーを重ねて作らない
    let exclude: Vec<_> = accounts::passkeys_of(&state.pool, &participant.user_id)
        .await?
        .iter()
        .filter_map(|j| serde_json::from_str::<Passkey>(j).ok())
        .map(|p| p.cred_id().clone())
        .collect();
    let (options, registration) = state
        .webauthn
        .start_passkey_registration(
            user_uuid,
            PASSKEY_NAME,
            PASSKEY_NAME,
            (!exclude.is_empty()).then_some(exclude),
        )
        .map_err(|e| anyhow::anyhow!("パスキーの登録を始められません: {e}"))?;
    let options = require_resident_key(options)?;
    let challenge_id = accounts::save_challenge(
        &state.pool,
        "register",
        Some(&participant.user_id),
        &json(&registration)?,
    )
    .await?;
    Ok((
        participant,
        Json(Started {
            challenge_id,
            options,
        }),
    ))
}

/// POST /api/auth/register/finish
pub async fn register_finish(
    State(state): State<AppState>,
    MaybeParticipant(user_id): MaybeParticipant,
    ApiJson(input): ApiJson<RegisterFinish>,
) -> Result<Json<Me>, ApiError> {
    let user_id = user_id.ok_or_else(invalid_challenge)?;
    if let Some(token) = &input.operator_token
        && !accounts::operator_invite_valid(&state.pool, token).await?
    {
        return Err(ApiError::validation(
            "invite_invalid",
            "運営者の招待のリンクが使えません（使用済みか期限切れ）",
        ));
    }
    let saved =
        accounts::take_challenge(&state.pool, &input.challenge_id, "register", Some(&user_id))
            .await?
            .ok_or_else(invalid_challenge)?;
    let registration: PasskeyRegistration = parse(&saved)?;
    let passkey = state
        .webauthn
        .finish_passkey_registration(&input.credential, &registration)
        .map_err(|_| invalid_passkey())?;
    accounts::add_passkey(
        &state.pool,
        &user_id,
        &credential_id(&passkey)?,
        &json(&passkey)?,
    )
    .await?;
    if let Some(token) = &input.operator_token {
        accounts::accept_operator_invite(&state.pool, token, &user_id).await?;
    }
    Ok(Json(accounts::status(&state.pool, &user_id).await?.into()))
}

/// POST /api/auth/login/start — 名前を入力しないパスキーでのログインを始める
pub async fn login_start(
    State(state): State<AppState>,
) -> Result<Json<Started<RequestChallengeResponse>>, ApiError> {
    let (options, authentication) = state
        .webauthn
        .start_discoverable_authentication()
        .map_err(|e| anyhow::anyhow!("ログインを始められません: {e}"))?;
    let challenge_id =
        accounts::save_challenge(&state.pool, "login", None, &json(&authentication)?).await?;
    Ok(Json(Started {
        challenge_id,
        options,
    }))
}

/// POST /api/auth/login/finish — その利用者に切り替え、この端末の匿名の参加者をまとめる
pub async fn login_finish(
    State(state): State<AppState>,
    MaybeParticipant(current): MaybeParticipant,
    ApiJson(input): ApiJson<LoginFinish>,
) -> Result<(Participant, Json<Me>), ApiError> {
    let saved = accounts::take_challenge(&state.pool, &input.challenge_id, "login", None)
        .await?
        .ok_or_else(invalid_challenge)?;
    let authentication: DiscoverableAuthentication = parse(&saved)?;
    let (user_uuid, _) = state
        .webauthn
        .identify_discoverable_authentication(&input.credential)
        .map_err(|_| invalid_passkey())?;
    let credential_id = serde_json::to_value(&input.credential.raw_id)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid_passkey)?;
    let (account, stored) = accounts::find_passkey(&state.pool, &credential_id)
        .await?
        .ok_or_else(invalid_passkey)?;
    if Uuid::parse_str(&account).ok() != Some(user_uuid) {
        return Err(invalid_passkey());
    }
    let mut passkey: Passkey = parse(&stored)?;
    let result = state
        .webauthn
        .finish_discoverable_authentication(
            &input.credential,
            authentication,
            &[DiscoverableKey::from(&passkey)],
        )
        .map_err(|_| invalid_passkey())?;
    let updated = passkey.update_credential(&result) == Some(true);
    accounts::touch_passkey(
        &state.pool,
        &credential_id,
        updated.then(|| json(&passkey)).transpose()?.as_deref(),
    )
    .await?;

    if let Some(current) = current
        && current != account
        && !accounts::status(&state.pool, &current).await?.registered
    {
        accounts::merge_anonymous(&state.pool, &current, &account).await?;
    }
    let status = accounts::status(&state.pool, &account).await?;
    Ok((Participant::switch_to(&state, account), Json(status.into())))
}

/// POST /api/auth/logout
pub async fn logout(jar: SignedCookieJar) -> (StatusCode, SignedCookieJar) {
    (StatusCode::NO_CONTENT, participant::logout(jar))
}
