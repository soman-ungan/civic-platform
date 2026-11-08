# 本番の動かし方（デプロイ）

本番は、サーバー1台の上で docker compose が4つのコンテナ（と、起動の前の1つ）を動かします（ADR 0001。計画は `docs/plans/2026-11-01-phase7-release.md`）。

| コンテナ | 中身 |
|---|---|
| `caddy` | 入口。HTTPS の証明書を自動で取り、`/api` を backend へ、ほかを frontend へ。公開の前はベーシック認証 |
| `backend` | API サーバーと運営のコマンド（同じバイナリ）。DB（SQLite）はボリューム `db` の `/data/civic.db` |
| `frontend` | Next.js（`server.js`） |
| `litestream` | SQLite を Cloudflare R2 に常に複製する |
| `restore` | 起動の前に1回だけ動く。DB が無ければバックアップから戻す（下の「バックアップと復元」） |

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
- backend のイメージに入れるのは、バイナリと `data/seeds/`、予算の PDF を読む `pdftotext`（poppler-utils と日本語の文字の対応表 poppler-data）だけです（マイグレーションはバイナリに埋め込み済み。evals・docs・テスト・開発用の seed は入れません）

## 更新とロールバック
```sh
cd deploy
docker compose pull            # IMAGE_TAG のイメージを取ってくる
docker compose up -d           # 入れ替える（backend は起動のときにマイグレーションを当てる）
```
- 戻すときは、`deploy/.env` の `IMAGE_TAG` を前のコミットの SHA にして `pull` と `up -d` をし直します
- **マイグレーションは戻りません。** DB の形を変える更新の前には、その時刻を控えておきます。戻すときはイメージと一緒に、DB もその時刻に戻します（下の「ある時点に戻す」）

## バックアップと復元（フェーズ7c）
Litestream が SQLite の変更を1秒ごとに Cloudflare R2 に送ります。まるごとの写しは1日1回で、`LITESTREAM_RETENTION`（既定 `168h` = 7日）のあいだ残り、その期間の中の時点に戻せます（直近ほど細かく選べます）。設定は `deploy/litestream.yml`。

### R2 の準備（初めに1回）
1. Cloudflare の R2 でバケットを作る（公開のアクセスはオフのまま）。場所の指定（location hint）はアジア太平洋に
2. 「API トークンを管理」で、**そのバケットだけ**に「オブジェクトの読み取りと書き込み」の権限のトークンを作る
3. `deploy/.env` に `LITESTREAM_BUCKET`・`LITESTREAM_ENDPOINT`（`https://<アカウント ID>.r2.cloudflarestorage.com`）・`LITESTREAM_ACCESS_KEY_ID`・`LITESTREAM_SECRET_ACCESS_KEY` を入れる
4. プライバシーポリシーに出す `SITE_BACKUP`（例: `Cloudflare, Inc.（米国）の R2（アジア太平洋）`）と `SITE_BACKUP_RETENTION` を埋める。消した情報は、`LITESTREAM_RETENTION` に写しの間隔（1日）を足した期間までバックアップに残るので、`168h` なら `8日`

R2 の値が空や間違いだと、`restore` が失敗して backend は起動しません（空の DB で始めないように）。

### 動いているかを見る
```sh
docker compose logs litestream | tail      # replicating to …・snapshot complete・compaction complete が出ていれば動いている
docker compose ps -a restore               # Exited (0)
```

### 新しいサーバーへ移す・ボリュームを失ったとき
何もしなくてよいです。`docker compose up -d` のとき、`restore` が DB の無いことを見て、R2 の最新から戻します（DB があるときと、バックアップがまだ無い初めてのデプロイでは何もしません）。**古いサーバーの litestream は先に止めます**（2台から同じ場所に送らないように）。

### ある時点に戻す（誤った操作・壊れた更新のあと）
時刻は UTC で書きます（日本時間から9時間引く）。今の DB は `/data/before-restore/` に残します。

```sh
cd deploy
docker compose stop backend litestream
# 1. 指定の時刻の DB を別の名前で作る
docker compose run --rm --no-deps restore \
  'litestream restore -timestamp 2026-11-03T02:00:00Z -o /data/restored.db /data/civic.db'
# 2. 入れ替える（Litestream の手元の記録も消す）
docker compose run --rm --no-deps restore \
  'mkdir -p /data/before-restore && mv /data/civic.db /data/civic.db-wal /data/civic.db-shm /data/before-restore/ 2>/dev/null; mv /data/restored.db /data/civic.db && rm -rf /data/.civic.db-litestream && chown -R 10001:10001 /data'
docker compose up -d
```
- `-timestamp` を外すと最新に戻ります
- 戻したあとの書き込みは、そのまま R2 に送られ、続きとして残ります（手元で、戻したあとに書き込み、ボリュームを消して最新から戻し、書き込みが残っていること・`integrity_check` が ok なことを確かめました）
- 確かめが済んだら `/data/before-restore/` を消します（`docker compose run --rm --no-deps restore 'rm -rf /data/before-restore'`）

### 戻せることを確かめる
公開の前（7e）にサーバーで一度、上の「ボリュームを失ったとき」を試します（`docker compose down` のあと `docker volume rm civic-platform_db`、`up -d`）。その後も、DB の形を変える大きな更新の前などに時々試します。

## 運営のコマンド
CLI は backend のコンテナの中で動かします（`docs/operations/cli.md` の `cargo run --` の代わりに `backend`）。

```sh
docker compose exec backend backend seed                 # data/seeds（開発用の seed は入りません）
docker compose exec backend backend add operator         # 運営者の招待のリンク
docker compose exec backend backend list topic -m otawara
```

## 資料を本番に入れる
本番のサーバーは、資料を自動では取り込みません（定期の実行は公開のあと）。入れ方は2つです。

**決算カード**: サーバーが総務省から取ってきます。サーバーでコマンドを打ちます。
```sh
docker compose exec backend backend fetch settlement -m otawara
docker compose exec backend backend import settlement -m otawara
```

**会議録・予算の概要**: 手元で原本から抽出の JSON を作り（共同作業の `--manual` か API。`docs/operations/cli.md`）、
`deploy/upload.sh` で本番に送って取り込みます。スクリプトは、原本と隣の `<名前>.extraction.json`・`<名前>.meta.json` を
backend のコンテナに送り、`import --dry-run` で検査します。`--apply` を付けると、検査のあとに下書きとして保存します。
送ったファイルは、終わったらコンテナから消します（原本は保持しない）。

```sh
# 送り先（~/.ssh/config に Host machikoe を書いておく。下）
export DEPLOY_SSH=machikoe DEPLOY_DIR='~/machikoe/deploy'

deploy/upload.sh minutes data/private/minutes/2026R08_0616_02-02.txt -m otawara            # 検査だけ
deploy/upload.sh --apply minutes data/private/minutes/2026R08_0616_02-02.txt -m otawara    # 下書きにする
# 抽出の JSON の名前が原本と違うときは --extraction で
deploy/upload.sh --apply --extraction data/private/budget/r8-extraction.json \
  budget data/private/budget/R8yosangaiyou.pdf -m otawara -y 2026 --url https://...
```
- 本番の DB に書くのは、サーバーの上の `import` だけです。引用が原文にあるか・氏名・AI の審査（キーがあれば）などの検査は、本番でもう一度通ります
- 下書きは、管理画面（`/operator`）で確かめて公開します。予算は `docker compose exec backend backend publish budget -m <slug> -y <年度>`
- `DEPLOY_SSH` が空なら、サーバーの上で動かしているものとして ssh を使いません（`DEPLOY_DIR` の既定はスクリプトのあるフォルダ）

`~/.ssh/config` の例（鍵はこのサーバーだけに使う）:
```
Host machikoe
  HostName <サーバーの IP>
  User ubuntu
  IdentityFile ~/.ssh/machikoe_ed25519
  IdentitiesOnly yes
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
バックアップの送り先も要ります。R2 を使わずに試すときは、S3 互換のサーバーを compose の上書きで足します（例: `rclone serve s3`。`LITESTREAM_ENDPOINT=http://s3:8080`）。

## まだ書いていないこと
- サーバーの準備（SSH・ファイアウォール・自動の更新・Docker）と、初めてのデプロイ（7e）
