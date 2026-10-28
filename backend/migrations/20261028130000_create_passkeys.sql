-- SPDX-License-Identifier: AGPL-3.0-only
-- パスキーでの認証（ADR 0006、docs/plans/2026-10-28-phase6a-passkeys.md）。
-- 保存するのはパスキーの公開鍵とその ID、ランダムな利用者の ID、日時だけ。名前・メールアドレス・電話番号は持たない

-- パスキー。id は credential ID（base64url）、passkey は webauthn-rs が出す JSON（公開鍵・署名の回数など）
CREATE TABLE passkeys (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users (id),
    passkey      TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    last_used_at TEXT
);

CREATE INDEX idx_passkeys_user ON passkeys (user_id);

-- 登録・ログインの途中の状態（数分で期限切れ。使ったら消す）
CREATE TABLE webauthn_challenges (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL CHECK (kind IN ('register', 'login')),
    user_id    TEXT REFERENCES users (id),
    state      TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

-- 運営者にするための、1回だけ使える登録の招待（CLI の add operator で作る）。トークンそのものは持たず、ハッシュだけ
CREATE TABLE operator_invites (
    token_hash TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at    TEXT
);

-- パスキーを初めて登録した日時（NULL なら匿名の参加者のまま。投票はできるが投稿はできない）
ALTER TABLE users ADD COLUMN registered_at TEXT;
-- 運営者か（画面からは変えられない。CLI から）
ALTER TABLE users ADD COLUMN is_operator BOOLEAN NOT NULL DEFAULT FALSE;

-- 本人の属性は集めない（ADR 0002）。使っていない列を消す
ALTER TABLE users DROP COLUMN age_group;
ALTER TABLE users DROP COLUMN gender;
ALTER TABLE users DROP COLUMN occupation;
