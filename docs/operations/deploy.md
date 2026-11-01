# 本番の動かし方（デプロイ）

本番は、サーバー1台の上で docker compose が4つ（予定）のコンテナを動かします（ADR 0001。計画は `docs/plans/2026-11-01-phase7-release.md`）。

| コンテナ | 中身 |
|---|---|
| `caddy` | 入口。HTTPS の証明書を自動で取り、`/api` を backend へ、ほかを frontend へ。公開の前はベーシック認証 |
| `backend` | API サーバーと運営のコマンド（同じバイナリ）。DB（SQLite）はボリューム `db` の `/data/civic.db` |
| `frontend` | Next.js（`server.js`） |
| `litestream` | SQLite を Cloudflare R2 に常に複製する（7c で足す） |

設定は `deploy/` にあります（`compose.yaml`・`Caddyfile`・`.env.example`）。

## 値（deploy/.env）
`deploy/.env.example` を `deploy/.env` に写して埋めます。`deploy/.env` はリポジトリに入れません。

- `SITE_DOMAIN`: ドメイン。HTTPS の証明書・パスキー・共有したときの URL に使います。**パスキーはドメインに結び付くので、公開のあとは変えません**（ADR 0006）
- `SESSION_SECRET`: `openssl rand -hex 32`
- `BASIC_AUTH=on`・`BASIC_AUTH_USER`・`BASIC_AUTH_HASH`: 公開の前のベーシック認証。ハッシュは `docker run --rm caddy:2 caddy hash-password --plaintext '…'` で作り、**`$` をすべて `$$` に書き換えて**から入れます（compose が `$` を変数と読むため）
- `SITE_*`: このサービスについて・規約・プライバシーポリシーに載せる値。frontend が**リクエストのときに読む**ので、変えたら `docker compose up -d frontend` で入れ直すだけです（作り直しは要りません）
- 空の値は「未設定」と同じに扱います

## 守り（フェーズ7b）
- **セキュリティのヘッダー**（Caddyfile）: HSTS（1年）・`X-Content-Type-Options`・`Referrer-Policy`・ほかのサイトへの埋め込みの禁止（`frame-ancestors 'none'`）・`Permissions-Policy`。`Server` は出さない
- **書き込みの回数の制限**（backend）: 同じ IP からの投票・投稿・提案・パスキーの手続きを数え、超えたら 429（`docs/design/api.md`）。Caddy は、信頼しない相手から来た `X-Forwarded-For` を捨てて接続元を入れ直すので、利用者がそのヘッダーを偽っても逃れられない
- **起動のときの確かめ**: `WEBAUTHN_ORIGIN` が https なのに `COOKIE_SECURE` が true でない、本番のドメインなのに http、のときは backend が起動しない。本番では `seed --dev`（【サンプル】の架空のデータ）を入れられない
- **アクセスの記録**: Caddy が `caddy_data` のボリュームの `access.log` に JSON で残し、`ACCESS_LOG_KEEP`（既定 `2160h` = 90日）を過ぎたら消す。プライバシーポリシーに出す `SITE_ACCESS_LOG_RETENTION` と同じ期間にする。Cookie と Authorization の値は記録されない（`REDACTED`）
- **コンテナのログ**: 1つ10MB・5つまで（compose の `x-logging`）
- **SQLite**: WAL・`synchronous=NORMAL`・書き込みの待ち 5秒。待ちを超えた（`database is locked`）エラーは、ログに `sqlite_busy=true` が付く。これが出始めたら PostgreSQL への移行の目安（ADR 0001）
  ```sh
  docker compose logs backend | grep 'sqlite_busy=true'
  ```

## イメージ
main に入ると、CI がイメージを作って GHCR に置きます（`.github/workflows/ci.yml` の `images`）。

- `ghcr.io/soman-ungan/civic-platform-backend:<コミットの SHA>` と `:latest`（frontend も同じ）
- frontend には、ビルドのときにソースの URL・サイトの URL（リポジトリの Actions の変数 `SITE_URL`）・コミットの SHA を埋め込みます。フッターの「ソースコード（AGPL）」は、動いているコミットのソースを開きます（AGPL 第13条）
- backend のイメージに入れるのは、バイナリと `data/seeds/` だけです（マイグレーションはバイナリに埋め込み済み。evals・docs・テスト・開発用の seed は入れません）

## 更新とロールバック
```sh
cd deploy
docker compose pull            # IMAGE_TAG のイメージを取ってくる
docker compose up -d           # 入れ替える（backend は起動のときにマイグレーションを当てる）
```
- 戻すときは、`deploy/.env` の `IMAGE_TAG` を前のコミットの SHA にして `pull` と `up -d` をし直します
- **マイグレーションは戻りません。** DB の形を変える更新の前には、バックアップから戻せることを確かめておきます（7c）

## 運営のコマンド
CLI は backend のコンテナの中で動かします（`docs/operations/cli.md` の `cargo run --` の代わりに `backend`）。

```sh
docker compose exec backend backend seed                 # data/seeds（開発用の seed は入りません）
docker compose exec backend backend add operator         # 運営者の招待のリンク
docker compose exec backend backend list topic -m otawara
```

## 手元で本番と同じ組み方を試す
443 以外のポートと `localhost` で動かせます（Caddy が手元用の証明書を作ります。ブラウザには警告が出ます）。

```sh
# deploy/.env の代わりに、試し用の値のファイルを使う
SITE_DOMAIN=localhost
PUBLIC_ORIGIN=https://localhost:8443
HTTP_PORT=8088
HTTPS_PORT=8443
IMAGE_TAG=local
SESSION_SECRET=...

docker compose -f deploy/compose.yaml --env-file <試し用の値> up -d --build
```

## まだ書いていないこと
- サーバーの準備（SSH・ファイアウォール・自動の更新・Docker）と、初めてのデプロイ（7e）
- バックアップと復元（7c）
