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
