// SPDX-License-Identifier: AGPL-3.0-only

//! 参加者のアカウント（ADR 0006）。パスキーの保存、匿名の参加者をアカウントにまとめること、
//! 登録・ログインの途中の状態、運営者の招待。
//! 保存するのはパスキーの公開鍵とその ID、ランダムな利用者の ID、日時だけ。WebAuthn のやりとりは `interface::api::auth`

use rand::RngExt;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::shared::util::{new_id, now_iso};

/// 登録・ログインの途中の状態を持つ時間（秒）
pub const CHALLENGE_TTL_SECS: i64 = 300;
/// 運営者の招待の有効期限（時間）
pub const OPERATOR_INVITE_TTL_HOURS: i64 = 24;

/// ログインの状態（`GET /auth/me`。利用者の ID は返さない）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Status {
    /// パスキーを登録している（投稿できる）
    pub registered: bool,
    pub operator: bool,
}

pub async fn status(pool: &SqlitePool, user_id: &str) -> Result<Status, sqlx::Error> {
    let row: Option<(bool, bool)> =
        sqlx::query_as("SELECT registered_at IS NOT NULL, is_operator FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(row
        .map(|(registered, operator)| Status {
            registered,
            operator,
        })
        .unwrap_or_default())
}

// ===== パスキー =====

/// その利用者の登録済みのパスキー（webauthn-rs の JSON）
pub async fn passkeys_of(pool: &SqlitePool, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT passkey FROM passkeys WHERE user_id = $1 ORDER BY created_at")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

/// パスキーを保存し、利用者を「登録済み」にする
pub async fn add_passkey(
    pool: &SqlitePool,
    user_id: &str,
    credential_id: &str,
    passkey: &str,
) -> Result<(), sqlx::Error> {
    let now = now_iso();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO passkeys (id, user_id, passkey, created_at) VALUES ($1, $2, $3, $4)")
        .bind(credential_id)
        .bind(user_id)
        .bind(passkey)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET registered_at = COALESCE(registered_at, $1) WHERE id = $2")
        .bind(&now)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// credential ID からパスキーを探す。(利用者の ID, パスキーの JSON)
pub async fn find_passkey(
    pool: &SqlitePool,
    credential_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT user_id, passkey FROM passkeys WHERE id = $1")
        .bind(credential_id)
        .fetch_optional(pool)
        .await
}

/// ログインに使ったパスキーの記録を更新する（署名の回数が変わったら JSON も）
pub async fn touch_passkey(
    pool: &SqlitePool,
    credential_id: &str,
    passkey: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE passkeys SET last_used_at = $1, passkey = COALESCE($2, passkey) WHERE id = $3",
    )
    .bind(now_iso())
    .bind(passkey)
    .bind(credential_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 匿名の参加者（その端末の Cookie）の投票・関わり方・投稿を、ログインしたアカウントに移し、匿名の参加者を消す。
/// 同じ意見への投票・同じ自治体との関わり方がアカウント側にあれば、アカウント側を残す。
/// 移さないと、同じ人が分析の中で2人に数えられてしまうため
pub async fn merge_anonymous(
    pool: &SqlitePool,
    anonymous: &str,
    account: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE votes SET user_id = $1
         WHERE user_id = $2
           AND statement_id NOT IN (SELECT statement_id FROM votes WHERE user_id = $1)",
    )
    .bind(account)
    .bind(anonymous)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM votes WHERE user_id = $1")
        .bind(anonymous)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE user_municipality_relations SET user_id = $1
         WHERE user_id = $2
           AND municipality_id NOT IN
               (SELECT municipality_id FROM user_municipality_relations WHERE user_id = $1)",
    )
    .bind(account)
    .bind(anonymous)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM user_municipality_relations WHERE user_id = $1")
        .bind(anonymous)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE statements SET user_id = $1 WHERE user_id = $2")
        .bind(account)
        .bind(anonymous)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM webauthn_challenges WHERE user_id = $1")
        .bind(anonymous)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM users WHERE id = $1 AND registered_at IS NULL")
        .bind(anonymous)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// 退会: パスキー・関わり方・登録済みの印・運営者の印を消し、消したパスキーの ID（credential ID）を返す。
/// 票と意見は残す（もとから誰のものかを外に出さず、パスキーが無くなれば、残る ID は本人にもたどれない乱数になる。
/// 分析も変えない）。利用者の行は、票と意見が指しているので残す
pub async fn delete_account(pool: &SqlitePool, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let credential_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM passkeys WHERE user_id = $1 ORDER BY created_at")
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await?;
    for sql in [
        "DELETE FROM passkeys WHERE user_id = $1",
        "DELETE FROM user_municipality_relations WHERE user_id = $1",
        "DELETE FROM webauthn_challenges WHERE user_id = $1",
        "UPDATE users SET registered_at = NULL, is_operator = FALSE WHERE id = $1",
    ] {
        sqlx::query(sql).bind(user_id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(credential_ids)
}

// ===== 登録・ログインの途中の状態 =====

/// 途中の状態を保存し、その ID を返す。期限の切れたものは、ついでに消す
pub async fn save_challenge(
    pool: &SqlitePool,
    kind: &str,
    user_id: Option<&str>,
    state: &str,
) -> Result<String, sqlx::Error> {
    let now = chrono::Utc::now();
    sqlx::query("DELETE FROM webauthn_challenges WHERE expires_at < $1")
        .bind(iso(now))
        .execute(pool)
        .await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO webauthn_challenges (id, kind, user_id, state, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(kind)
    .bind(user_id)
    .bind(state)
    .bind(iso(now + chrono::Duration::seconds(CHALLENGE_TTL_SECS)))
    .execute(pool)
    .await?;
    Ok(id)
}

/// 途中の状態を取り出して消す（1回だけ使える）。種類・利用者が違うもの、期限切れのものは None
pub async fn take_challenge(
    pool: &SqlitePool,
    id: &str,
    kind: &str,
    user_id: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>, String, String)> = sqlx::query_as(
        "DELETE FROM webauthn_challenges WHERE id = $1 AND kind = $2
         RETURNING user_id, state, expires_at",
    )
    .bind(id)
    .bind(kind)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|(owner, state, expires_at)| {
        (owner.as_deref() == user_id && expires_at >= iso(chrono::Utc::now())).then_some(state)
    }))
}

fn iso(t: chrono::DateTime<chrono::Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

// ===== 運営者 =====

fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 運営者にするための招待を作り、トークンを返す（DB にはハッシュだけを残す）
pub async fn create_operator_invite(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let bytes: [u8; 32] = rand::rng().random();
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO operator_invites (token_hash, created_at, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(token_hash(&token))
    .bind(iso(now))
    .bind(iso(now + chrono::Duration::hours(OPERATOR_INVITE_TTL_HOURS)))
    .execute(pool)
    .await?;
    Ok(token)
}

/// 招待がまだ使えるか（使われておらず、期限内）
pub async fn operator_invite_valid(pool: &SqlitePool, token: &str) -> Result<bool, sqlx::Error> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT token_hash FROM operator_invites
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at >= $2",
    )
    .bind(token_hash(token))
    .bind(now_iso())
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

/// 招待を使って、その利用者を運営者にする。使えない招待なら false
pub async fn accept_operator_invite(
    pool: &SqlitePool,
    token: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let used = sqlx::query(
        "UPDATE operator_invites SET used_at = $1
         WHERE token_hash = $2 AND used_at IS NULL AND expires_at >= $1",
    )
    .bind(now_iso())
    .bind(token_hash(token))
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if used == 0 {
        return Ok(false);
    }
    sqlx::query("UPDATE users SET is_operator = TRUE WHERE id = $1 AND registered_at IS NOT NULL")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

#[derive(Debug, sqlx::FromRow)]
pub struct Operator {
    pub id: String,
    pub registered_at: Option<String>,
    pub passkeys: i64,
}

pub async fn list_operators(pool: &SqlitePool) -> Result<Vec<Operator>, sqlx::Error> {
    sqlx::query_as(
        "SELECT u.id, u.registered_at,
                (SELECT COUNT(*) FROM passkeys p WHERE p.user_id = u.id) AS passkeys
         FROM users u WHERE u.is_operator ORDER BY u.registered_at",
    )
    .fetch_all(pool)
    .await
}

/// 運営者から外す。見つからなければ false
pub async fn remove_operator(pool: &SqlitePool, user_id: &str) -> Result<bool, sqlx::Error> {
    let n = sqlx::query("UPDATE users SET is_operator = FALSE WHERE id = $1 AND is_operator")
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(n > 0)
}
