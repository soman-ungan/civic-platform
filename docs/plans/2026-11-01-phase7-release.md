# フェーズ7 リリースの準備

## Context
全機能がそろったので、リリースの準備に入る（ADR 0001 の改訂どおり、デプロイの基盤はここでまとめて作る）。
2026-10-31〜11-01 ユーザーと決めたこと:
- デプロイ先はまだ決めない。どこでも使える部分（7a〜7c）を先に作り、そのあとで料金を調べ直して選ぶ
- ステージングは作らず、本番のサーバーにベーシック認証をかけて本人だけで確かめてから外す
- 運営者への知らせ（確認待ちが溜まったとき）は公開のあとで考える。それまでは管理画面を毎日開く
- ドメインはこれから取得する（パスキーはドメインに結び付くので、公開のあとは変えない）
- 本番の DB は当面 SQLite。PostgreSQL への移行は目安（下）で判断する（ADR 0001 に記録）

## やること

### 7a. コンテナ化（デプロイ先によらない）
- backend: 多段のビルドで小さな実行イメージ。マイグレーションはバイナリに埋め込み済み、イメージに入れるのは `data/seeds/` だけ（evals・docs・テスト・開発用の seed は入れない）
- frontend: Next.js の `output: "standalone"`（`node_modules/next/dist/docs/` で確かめる）。`NEXT_PUBLIC_*` はビルドの引数
- compose: backend・frontend・Caddy・Litestream。Caddy は HTTPS の自動の証明書、`/api` を backend へ、ほかを frontend へ。ベーシック認証は環境変数でオン・オフ
- 動いているコミット（AGPL 第13条）: イメージにコミットの SHA を埋め込み、フッターのソースのリンクからそのコミットを開ける
- CI: main でイメージを作り GHCR に置く
- 手元で `docker compose up` して確かめる

### 7b. 本番に向けた守り（アプリ側）
- 回数の制限: 投票・投稿・提案・パスキーの登録とログインを、同じ IP からの回数で制限する
- 起動のときの確かめ: 本番の設定（`WEBAUTHN_ORIGIN` が https）なのに `COOKIE_SECURE` がオフなどの食い違いでは起動しない。本番の設定では開発用の seed を入れられない
- セキュリティのヘッダー（Caddy）: HSTS・`X-Content-Type-Options`・`Referrer-Policy`・ほかのサイトへの埋め込みの禁止
- 送ってくるデータの大きさの上限を確かめる
- Caddy のアクセスの記録の保存期間を、プライバシーポリシーの `SITE_ACCESS_LOG_RETENTION` と合わせる
- SQLite の本番の設定: 書き込みの待ち時間（busy_timeout）、Litestream と使うときの推奨の設定、`database is locked` が出たらログで分かるように

### 7c. バックアップと復元
- Litestream で SQLite を Cloudflare R2 に常に複製する
- 復元の手順を文書にし、手元で一度復元して確かめる

### 7d. デプロイ先とドメインを決めて契約（ユーザー）
- 7a〜7c のあとで、料金と条件を調べ直して比べる。ドメインの候補を一緒に考える

### 7e. サーバーの準備と、ベーシック認証つきの仮公開
- SSH は鍵だけ・ファイアウォール・自動のセキュリティ更新・Docker
- デプロイとロールバックの手順を `docs/operations/deploy.md` に
- seed、運営者のパスキーの登録、本物の会議録と予算の取り込みと公開、実機での確認、本番での Lighthouse、サーバーでの復元の確かめ

### 7f. 公開
- `.env` の `SITE_*` を埋め、規約とプライバシーポリシーを専門家に確かめてもらう。議員の読みと任期の【要確認】を確かめる。収支を公開する場所を決める（`SITE_FINANCE_REPORT_URL`）
- リポジトリを Public にし、ベーシック認証を外し、外からの死活監視（無料のもの）を付ける

### 公開のあとに回すもの
- 運営者への知らせ
- 収集の定期実行（会議録は議会の書面の許可まで手でコピー。決算・予算は年1回で、手でコマンドを実行。手順は 7e の文書に）
- 寄付の受け付け

### PostgreSQL への移行の目安（ADR 0001）
- サーバーを2台以上にしたい／`database is locked` がログに出始める／フェーズ8 で参加者が数万人の規模になる／時点を指定した復元や DB の外部サービスに任せたい
- 作業は大きい（接続の型 `SqlitePool` が38ファイル・約200か所。個々の SQL を PostgreSQL で動かす確かめ、データの移し替えとリハーサル）。それだけで1つの計画にする

## ライセンス
Caddy・Litestream は Apache-2.0。ベースのイメージは採用するときに確かめる。

## 実装時の変更点
### 7a
- 運営者ごとの値（`SITE_*`）は、ビルドのときに静的な HTML に焼き込まれていた。イメージは CI で作って GHCR に置くので、個人の値が入らないよう、リクエストのときに読むようにした（`lib/site.ts` の `siteValue`、`SiteValue` は `connection()` で待つ小さな穴。ページのほかは静的なまま）。問い合わせのメールも `NEXT_PUBLIC_CONTACT_EMAIL` から、サーバーだけで読む `SITE_CONTACT_EMAIL` に
- backend の設定は、空の値を未設定と同じに扱う（compose は未設定の変数を空で渡すため。空のモデル名で API を呼ばないように）
- パスキー（webauthn-rs）が OpenSSL を使うので、backend のイメージに libssl を入れた（OpenSSL 3 は Apache-2.0。外部への HTTP は rustls のまま）
- 公開の前のベーシック認証は Caddy の snippet を `BASIC_AUTH=on|off` で切り替える。ハッシュの `$` は `$$` に書く
- 手元で試すため、ポート（`HTTP_PORT`・`HTTPS_PORT`）と `PUBLIC_ORIGIN` を変えられるようにした
- 動いているコミットは、フッターの「ソースコード（AGPL）」がそのコミットのツリーを開き、about に短い SHA を出す（`lib/source.ts`）
- 確かめ: 手元で compose を `https://localhost:8443` で動かし、ベーシック認証・各ページ・コンテナの中の seed・運営者の招待からのパスキーの登録（headless の Chromium の仮想の認証器）・Cookie の Secure と HttpOnly・リクエストのときの `SITE_*`・コミットのリンクを確かめた。イメージは backend 108MB・frontend 270MB

### 7b
- 回数の制限は、新しい crate を入れず、backend の中で時間の窓ごとに数える（`interface/api/rate_limit.rs`。サーバー1台・再起動で消えてよい）。スマホの回線は同じ IP を分け合うので、上限は大きめ（投票300/分・投稿30/分・提案20/時・パスキーの手続きと関わり方60/分）。テストでは数えない（`AppState::new` は無効。`with_rate_limiter` で有効に）
- 接続元の IP は、`TRUST_FORWARDED_FOR=true`（compose）のとき Caddy が付ける `X-Forwarded-For` の一番右、それ以外は接続元のアドレス（`into_make_service_with_connect_info`）
- 本番の判定は「`WEBAUTHN_RP_ID` が localhost 以外」（`Config::is_production`）。起動の確かめ（`check_for_serving`）と `seed --dev` の拒否に使う
- アクセスの記録の期間は、Caddy の `roll_keep_for`（`ACCESS_LOG_KEEP`）で時間で決める（大きさで回すのではなく）。`deploy/.env.example` の `SITE_ACCESS_LOG_RETENTION` の例を「90日」にそろえた
- 送ってくる本文の上限は 64KB（`DefaultBodyLimit`）
- 確かめ: 手元の compose で、ヘッダー・偽の `X-Forwarded-For` を付けても31回目で 429・アクセスの記録で Authorization が `REDACTED`・食い違った設定で backend が起動しない・本番で `seed --dev` が拒まれる、を確かめた


### 7c
- Litestream は v0.5.17 の公式のイメージ（Apache-2.0）を、backend と同じ uid 10001 で動かす（`-shm` などを backend が書けるように）
- 起動の前に1回だけ動く `restore` を足した。DB が無ければ R2 の最新から戻し（`-if-db-not-exists -if-replica-exists`）、backend はその成功を待つ。新しいサーバーへの移行やボリュームを失ったときに手で戻さなくてよく、R2 の値が空や間違いなら backend が起動しない（空の DB で始めない）。新しいボリュームは root の持ち主で作られるので、root で戻してから `chown`
- まるごとの写しは1日1回、`LITESTREAM_RETENTION`（既定 168h = 7日）残す。消した情報がバックアップに残ることをプライバシーポリシーに書いた（`SITE_BACKUP`・`SITE_BACKUP_RETENTION`。残る最長は保存の期間＋1日で、既定なら8日）
- 確かめ: R2 の代わりに `rclone serve s3` を compose の上書きで足し、手元で、複製・ボリュームを消したあとの自動の復元（API の応答が同じ）・戻したあとの書き込みの複製・ある時点への復元（`-timestamp`）とそのあとの書き込み → ボリュームを消して最新から戻す（件数が合い `integrity_check` が ok）・R2 の値が空や届かないときに `restore` が失敗すること、を確かめた。本物の R2 での確かめは 7e で
