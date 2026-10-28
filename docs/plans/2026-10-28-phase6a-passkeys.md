# フェーズ6a: パスキーでの認証の土台

## Context
フェーズ6（認証）を 6a〜6e に分けた（2026-10-28 ユーザーと決定）。
- 6a 認証の土台（この計画） / 6b 参加の決まり（投稿の上限・分析を開く条件・似た意見の案内） / 6c 管理画面（下書きのトピック・確認待ちの意見・市民からの提案） / 6d 市民からのトピックの提案 / 6e 1人を見分ける目印（乱用が見えたら）
- 方針は ADR 0006: 読む・投票はログインなし、投稿と提案はパスキーの登録が要る。保存するのは公開鍵と ID と日時だけ

## やること

### backend
1. **マイグレーション**
   - `passkeys`（`id` = credential ID、`user_id`、`passkey`（webauthn-rs が出す JSON。公開鍵など）、`created_at`、`last_used_at`）
   - `users.registered_at`（パスキーを初めて登録した日時。NULL なら匿名のまま）、`users.is_operator`（運営者）
   - `users` の使わない列（`age_group`・`gender`・`occupation`）を消す（グランドデザインの「将来削除する候補」）。SQLite で CHECK 付きの列を消せない場合は、テーブルを作り直す。PostgreSQL 互換の CI で確かめる
   - `webauthn_challenges`（登録とログインの途中の状態。数分で期限切れ）
2. **WebAuthn**: `webauthn-rs`（MPL-2.0。AGPL と両立。パスキーの登録とログインの検証）を使う。設定は `WEBAUTHN_RP_ID`（既定 `localhost`）と `WEBAUTHN_ORIGIN`（既定 `http://localhost:3000`）
3. **API**（すべて `/api/auth` の下）
   - `POST /auth/register/start` → `finish`: **今の匿名の参加者にパスキーを付ける**（投票の記録をそのまま引き継ぐ）。Cookie が無ければ参加者を作る
   - `POST /auth/login/start` → `finish`: パスキーで、そのアカウントの Cookie に切り替える
     - その端末の匿名の参加者に投票があれば、アカウントに移す（同じ意見への投票がアカウント側にあれば、アカウント側を残す）。関わり方（`user_municipality_relations`）も、アカウント側に無いものだけ移す。移したあと、匿名の参加者は消す
   - `POST /auth/logout`: Cookie を消す（その端末では、新しい匿名の参加者として続けられる）
   - `GET /auth/me`: ログインしているか（`registered`）と運営者か。利用者の ID は返さない
4. **投稿にパスキーを要るようにする**: `POST .../statements` は、`registered_at` が無ければ 401（`code: "registration_required"`）。投票と関わり方の登録は今のまま
5. **運営者**: CLI `add operator`（運営者にするための、1回だけ使える登録のリンクを出す。リンクから登録したパスキーのアカウントが運営者になる）と `list operator`・`remove operator --id`。運営者の画面は 6c
   - 段階の分類（docs/plans/2026-10-28-cli-verbs.md）では、運営者は段階2の対象（`operator`）として足す
6. パスキーの表示名は個人の情報を含まない固定の文（例「大田原の参加者」ではなく、自治体名も入れない「civic-platform の参加者」）

### frontend
- WebAuthn の呼び出しは `@simplewebauthn/browser`（MIT。ブラウザの API の JSON の変換）を使う
- 意見の投稿フォーム: 未登録なら「意見を投稿するには、パスキーの登録が要ります」と、登録のボタン（端末の指紋・顔・PIN で作る、と短く説明）。登録したらそのまま投稿できる
- ヘッダーかフッターに「ほかの端末でログイン（パスキー）」と「ログアウト」。ログイン中かは `GET /auth/me`
- 投稿者が誰かは、画面のどこにも出さない（今までどおり）

### ドキュメント
- ADR 0006、CLAUDE.md（認証・API・環境変数）、`docs/cli.md`（operator）、`.env.example`（`WEBAUTHN_RP_ID`・`WEBAUTHN_ORIGIN`）、グランドデザイン §6 の注記（SNS ログインからパスキーへ）

## 確認
- `cargo test`・clippy・fmt。テストは `webauthn-authenticator-rs` のソフトウェアのパスキー（MPL-2.0。テストの中だけで使う）で、登録→ログイン→投稿を通す（実際の端末は使わない）
  - 匿名で投票してから登録すると、投票が引き継がれる
  - 別の端末（別の Cookie）でログインすると、その端末の匿名の投票がアカウントに移り、重なる意見はアカウント側が残る
  - 未登録では投稿できず（401）、投票はできる
  - チャレンジの使い回し・期限切れは失敗する
  - `GET /auth/me` が利用者の ID を返さない
  - 自治体の取り違えが無い（2自治体の seed）
- frontend の lint・typecheck・build
- 開発サーバーで、実際のブラウザ（Chrome の仮想の認証器、または手元の端末）で登録・ログイン・投稿を確かめる（ユーザーにも見てもらう）

## しないこと（後の段階）
- 投稿の上限・分析を開く条件・似た意見の案内（6b）、管理画面（6c）、提案（6d）、目印（6e）
- 本番のドメインでの設定（フェーズ7）

## 実装時の変更点
- **登録の指示を書き換えた**: webauthn-rs 0.5.5 の `start_passkey_registration` は「端末の側に持つ形」（discoverable credential）を求めないので、ブラウザに渡す JSON の `residentKey` を `required` にした（名前を入力しないログインのため）。型（`ResidentKeyRequirement`）は prelude に無いので、依存を足さずに JSON で書き換えた
- **ログインは `start_discoverable_authentication`**（`conditional-ui` 機能）。途中の状態を DB に置くため `danger-allow-state-serialisation` 機能も使う
- **テストのソフトウェアのパスキーは discoverable credential を作れない**ので、テストの中だけ登録の指示を外し、ログインでは登録したパスキーを指定して利用者の ID を応答に入れる（`tests/auth.rs` の冒頭に理由）
- **運営者の招待**は `operator_invites`（トークンのハッシュ・期限・使った日時）。リンクの先は `/operator/join`
- **frontend**: `@simplewebauthn/browser` 14.0.0（MIT）。投稿フォームで未登録ならパスキーの案内（`passkey-prompt.tsx`）、フッターに「パスキーでログイン」「ログアウト」（`account-links.tsx`）、`/operator/join`。`apiFetch` は 204 の応答も受け取れるようにした
- **実際のブラウザでの確認**: この環境にブラウザが無いため、ユーザーに手元のブラウザで確かめてもらう
