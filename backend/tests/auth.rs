// SPDX-License-Identifier: AGPL-3.0-only

//! パスキーでの登録とログイン（ADR 0006）。ソフトウェアのパスキー（webauthn-authenticator-rs）で、
//! 実際の端末を使わずに API を通して確かめる。
//!
//! ソフトウェアのパスキーは「端末の側に持つ形」（discoverable credential）を作れず、ログインの応答に利用者の ID
//! （user handle）を入れない。そこでテストの中だけ、本物の端末と同じ動きになるよう補う:
//! - 登録: 端末の側に持つ形を求める指示を外してから渡す（サーバーは登録の確認でこの指示を見ない）
//! - ログイン: 登録したパスキーを指定してから渡し、応答に利用者の ID を入れる（利用者の ID は署名の対象ではない）

mod common;

use axum::http::StatusCode;
use backend::interface::api::AppState;
use backend::records::accounts;
use common::{TestResponse, request};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{
    Base64UrlSafeData, CreationChallengeResponse, RequestChallengeResponse, Url,
};

const ORIGIN: &str = "http://localhost:3000";
const OTAWARA: &str = "/api/municipalities/otawara";

async fn setup() -> (SqlitePool, AppState) {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO statements (id, topic_id, body, status, created_at) VALUES
            ('a1', 't1', '承認済み1', 'approved', '2026-10-26T00:00:00Z'),
            ('a2', 't1', '承認済み2', 'approved', '2026-10-26T00:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = common::test_state(pool.clone());
    (pool, state)
}

/// 端末（ソフトウェアのパスキー）
struct Device {
    authenticator: WebauthnAuthenticator<SoftPasskey>,
    /// 登録したパスキー（credential ID, 利用者の ID）
    registered: Option<(Base64UrlSafeData, Base64UrlSafeData)>,
}

impl Device {
    fn new() -> Self {
        Self {
            authenticator: WebauthnAuthenticator::new(SoftPasskey::new(true)),
            registered: None,
        }
    }
}

/// 匿名の参加者として関わり方を登録し、Cookie を返す
async fn anonymous(state: &AppState) -> String {
    let res = request(
        state,
        "PUT",
        &format!("{OTAWARA}/relation"),
        None,
        Some(json!({"residency_type": "resident"})),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
    res.cookie.unwrap()
}

async fn vote(state: &AppState, cookie: &str, sid: &str, value: i64) {
    let res = request(
        state,
        "POST",
        &format!("{OTAWARA}/topics/t1/statements/{sid}/votes"),
        Some(cookie),
        Some(json!({"value": value})),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
}

/// パスキーを登録する。新しく参加者が作られたら、その Cookie も返す
async fn register(
    state: &AppState,
    device: &mut Device,
    cookie: Option<&str>,
    operator_token: Option<&str>,
) -> (TestResponse, Option<String>) {
    let started = request(state, "POST", "/api/auth/register/start", cookie, None).await;
    assert_eq!(started.status, StatusCode::OK, "{:?}", started.json);
    let cookie = started.cookie.clone().or(cookie.map(str::to_owned));
    let mut options = started.json["options"].clone();
    assert_eq!(
        options["publicKey"]["authenticatorSelection"]["residentKey"],
        "required"
    );
    // パスキーの表示名に個人の情報を入れない（作った日だけ）
    let name = options["publicKey"]["user"]["name"].as_str().unwrap();
    assert!(
        name.starts_with("civic-platform（") && name.ends_with(" 作成）"),
        "{name}"
    );
    options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = false.into();
    options["publicKey"]["authenticatorSelection"]
        .as_object_mut()
        .unwrap()
        .remove("residentKey");
    let options: CreationChallengeResponse = serde_json::from_value(options).unwrap();
    let user_id = options.public_key.user.id.clone();
    let credential = device
        .authenticator
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    device.registered = Some((credential.raw_id.clone(), user_id));
    let finished = request(
        state,
        "POST",
        "/api/auth/register/finish",
        cookie.as_deref(),
        Some(json!({
            "challenge_id": started.json["challenge_id"],
            "credential": credential,
            "operator_token": operator_token,
        })),
    )
    .await;
    (finished, cookie)
}

/// 登録したパスキーでログインする
async fn login(state: &AppState, device: &mut Device, cookie: Option<&str>) -> TestResponse {
    let started = request(state, "POST", "/api/auth/login/start", cookie, None).await;
    assert_eq!(started.status, StatusCode::OK, "{:?}", started.json);
    let mut options = started.json["options"].clone();
    // 名前を入力しないログインでは、サーバーはパスキーを指定しない
    assert_eq!(options["publicKey"]["allowCredentials"], json!([]));
    let (credential_id, user_id) = device.registered.clone().unwrap();
    options["publicKey"]["allowCredentials"] = json!([{"type": "public-key", "id": credential_id}]);
    let options: RequestChallengeResponse = serde_json::from_value(options).unwrap();
    let mut credential = device
        .authenticator
        .do_authentication(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    credential.response.user_handle = Some(user_id);
    request(
        state,
        "POST",
        "/api/auth/login/finish",
        cookie,
        Some(json!({
            "challenge_id": started.json["challenge_id"],
            "credential": credential,
        })),
    )
    .await
}

async fn me(state: &AppState, cookie: Option<&str>) -> Value {
    request(state, "GET", "/api/auth/me", cookie, None)
        .await
        .json
}

async fn post_statement(state: &AppState, cookie: &str) -> StatusCode {
    request(
        state,
        "POST",
        &format!("{OTAWARA}/topics/t1/statements"),
        Some(cookie),
        Some(json!({"body": "公園をもっと増やしてほしい"})),
    )
    .await
    .status
}

async fn votes_of(pool: &SqlitePool, cookie_user: &str) -> Vec<(String, i64)> {
    sqlx::query_as(
        "SELECT statement_id, vote_value FROM votes WHERE user_id = $1 ORDER BY statement_id",
    )
    .bind(cookie_user)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn only_registered_user(pool: &SqlitePool) -> String {
    sqlx::query_scalar("SELECT id FROM users WHERE registered_at IS NOT NULL")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn registering_keeps_votes_and_allows_posting() {
    let (pool, state) = setup().await;
    let cookie = anonymous(&state).await;
    vote(&state, &cookie, "a1", 1).await;
    assert_eq!(
        post_statement(&state, &cookie).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        me(&state, Some(&cookie)).await,
        json!({"registered": false, "operator": false})
    );

    let mut device = Device::new();
    let (res, new_cookie) = register(&state, &mut device, Some(&cookie), None).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
    assert_eq!(
        new_cookie.as_deref(),
        Some(cookie.as_str()),
        "参加者は変わらない"
    );
    assert_eq!(res.json, json!({"registered": true, "operator": false}));
    // 利用者の ID はどこにも返さない
    common::assert_no_key(&res.json, "id");
    common::assert_no_key(&me(&state, Some(&cookie)).await, "user_id");

    // 投票はそのまま引き継がれ、投稿できるようになる
    let account = only_registered_user(&pool).await;
    assert_eq!(votes_of(&pool, &account).await, [("a1".to_string(), 1)]);
    assert_eq!(post_statement(&state, &cookie).await, StatusCode::CREATED);
    // 保存したのは公開鍵などのパスキーと日時だけ
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passkeys WHERE user_id = $1")
        .bind(&account)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn logging_in_on_another_device_merges_its_anonymous_votes() {
    let (pool, state) = setup().await;
    // 端末A: 匿名で a1 に反対してから登録（アカウント）
    let cookie_a = anonymous(&state).await;
    vote(&state, &cookie_a, "a1", -1).await;
    let mut device = Device::new();
    register(&state, &mut device, Some(&cookie_a), None).await;
    let account = only_registered_user(&pool).await;

    // 端末B: 匿名で a1 に賛成・a2 に賛成
    let cookie_b = anonymous(&state).await;
    vote(&state, &cookie_b, "a1", 1).await;
    vote(&state, &cookie_b, "a2", 1).await;
    let anonymous_b: String = sqlx::query_scalar(
        "SELECT id FROM users WHERE registered_at IS NULL ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // 端末Bでパスキーでログインすると、アカウントに切り替わる
    let res = login(&state, &mut device, Some(&cookie_b)).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
    assert_eq!(res.json, json!({"registered": true, "operator": false}));
    let switched = res.cookie.expect("Cookie を切り替える");
    assert_eq!(me(&state, Some(&switched)).await["registered"], true);
    assert_eq!(post_statement(&state, &switched).await, StatusCode::CREATED);

    // a1 はアカウント側（反対）を残し、a2 は移る。端末Bの匿名の参加者は消える
    assert_eq!(
        votes_of(&pool, &account).await,
        [("a1".to_string(), -1), ("a2".to_string(), 1)]
    );
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(&anonymous_b)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
async fn challenges_are_single_use_and_expire() {
    let (pool, state) = setup().await;
    let mut device = Device::new();
    let (_, cookie) = register(&state, &mut device, None, None).await;
    let cookie = cookie.unwrap();

    // ログインの途中の状態は1回だけ使える
    let started = request(&state, "POST", "/api/auth/login/start", None, None).await;
    let challenge_id = started.json["challenge_id"].as_str().unwrap().to_owned();
    let taken = accounts::take_challenge(&pool, &challenge_id, "login", None)
        .await
        .unwrap();
    assert!(taken.is_some());
    assert!(
        accounts::take_challenge(&pool, &challenge_id, "login", None)
            .await
            .unwrap()
            .is_none()
    );

    // 種類の違うもの・期限の切れたものは使えない
    let started = request(
        &state,
        "POST",
        "/api/auth/register/start",
        Some(&cookie),
        None,
    )
    .await;
    let challenge_id = started.json["challenge_id"].as_str().unwrap().to_owned();
    assert!(
        accounts::take_challenge(&pool, &challenge_id, "login", None)
            .await
            .unwrap()
            .is_none()
    );
    let started = request(&state, "POST", "/api/auth/login/start", None, None).await;
    let challenge_id = started.json["challenge_id"].as_str().unwrap().to_owned();
    sqlx::query(
        "UPDATE webauthn_challenges SET expires_at = '2000-01-01T00:00:00.000Z' WHERE id = $1",
    )
    .bind(&challenge_id)
    .execute(&pool)
    .await
    .unwrap();
    let res = request(
        &state,
        "POST",
        "/api/auth/login/finish",
        None,
        Some(json!({"challenge_id": challenge_id, "credential": {
            "id": "x", "rawId": "eA", "type": "public-key",
            "response": {"authenticatorData": "eA", "clientDataJSON": "eA", "signature": "eA"}
        }})),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["code"], "challenge_invalid");
}

#[tokio::test]
async fn operator_invite_is_used_once() {
    let (pool, state) = setup().await;
    let token = accounts::create_operator_invite(&pool).await.unwrap();

    let mut device = Device::new();
    let (res, _) = register(&state, &mut device, None, Some(&token)).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json);
    assert_eq!(res.json, json!({"registered": true, "operator": true}));
    // DB にはトークンそのものを持たない
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM operator_invites")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, token);

    // 同じ招待は2度使えない
    let mut other = Device::new();
    let (res, _) = register(&state, &mut other, None, Some(&token)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["code"], "invite_invalid");
    assert_eq!(accounts::list_operators(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn logout_clears_the_cookie() {
    let (_pool, state) = setup().await;
    let mut device = Device::new();
    let (_, cookie) = register(&state, &mut device, None, None).await;
    let res = request(&state, "POST", "/api/auth/logout", cookie.as_deref(), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let cleared = res.cookie.unwrap();
    assert!(cleared.ends_with('='), "{cleared}");
    assert_eq!(me(&state, None).await["registered"], false);
}
