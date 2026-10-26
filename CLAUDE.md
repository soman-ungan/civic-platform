# CLAUDE.md — 地方行政可視化＆市民参加プラットフォーム（多自治体対応）

このファイルは Claude Code がこのリポジトリで作業する際の指針です。
詳細な設計は `docs/grand-design.md` を参照すること（設計判断で迷ったら必ず読み直す）。

## プロジェクト概要
- 自治体の議事録・予算・決算を可視化し、Polis の思想に基づく合意形成の場を提供する Web アプリ。
- **多自治体対応**: 行政区分（都道府県 → 市区町村 → 地区）を選択可能な構造。栃木県大田原市がパイロット自治体。
- 主対象は 10〜30 代。スマホファースト。
- 個人開発・ワンオペ運用。自動化と保守のしやすさを最優先する。
- Polis を丸ごとフォークはせず、データ構造・画面設計・数理ロジック（PCA + クラスタリング）を Rust × Next.js に移植する。
  - 数理・統計処理は Polis の正式なアルゴリズム（Clojure 実装）を移植したもの（`backend/src/analysis/`）。方針・Polis との違い・一致テストの結果は `docs/polis-math.md`。変更するときは `cargo test --test polis_parity` で Polis との一致を確かめる。

## ライセンス（AGPL-3.0-only）
- 本プロジェクトは「行政の透明性」を掲げるため、サービス自体も透明であることを最優先し、**GNU AGPL v3（AGPL-3.0-only）** とする。本家 Polis（AGPL-3.0-only）と揃え、Polis の数理処理を移植できるようにするため（「or later」は付けない）。
- ルートに `LICENSE`（AGPLv3 全文）を置く。`backend/Cargo.toml` は `license = "AGPL-3.0-only"`、`frontend/package.json` は `"license": "AGPL-3.0-only"` とする。
- 新規ソースファイルの先頭に SPDX ヘッダーを付ける: `// SPDX-License-Identifier: AGPL-3.0-only`（SQL は `--`）。
- Polis（AGPL-3.0）由来のコードやロジックを取り込む場合は、元ファイルの出典と著作権表示を残し、`NOTICE` に記載する。
- **AGPL 第13条への対応**: フロントの全ページのフッターに、ソースコード（公開リポジトリ）へのリンクを表示する。URL は環境変数 `NEXT_PUBLIC_SOURCE_URL` から読む。
- **GitHub リポジトリは開発中は Private。本番公開の前に必ず Public に切り替える**（Private のままではフッターのソースリンクを利用者が開けず、第13条を満たせない）。
- 本番で動かすコードは必ず公開リポジトリと一致させる。秘密情報（API キー、管理者向けの値）はコードに書かず `.env` に置く。
- 依存ライブラリは AGPLv3 と両立するもの（MIT / Apache-2.0 / BSD / MPL-2.0 / LGPL / GPLv3 等）に限る。追加時にライセンスを確認し、両立しないもの（ライセンス不明、非商用限定など）は使わない。

## リポジトリ構成（モノレポ）
```
civic-platform/
├── backend/              Rust / axum / sqlx（API サーバー＋収集バッチ）
│   └── src/collectors/   自治体・議事録システム別の収集アダプタ
├── frontend/             Next.js (App Router) / TypeScript / Tailwind CSS
├── data/                 SQLite ファイル（*.db は git 管理しない）
│   └── seeds/            自治体・地区マスタの初期データ（git 管理する）
└── docs/                 設計書・ADR
```

## 多自治体対応の原則（最重要）
- **大田原市をコードにハードコードしない。** 自治体名・URL・地区名・人口はすべて DB のマスタ／seed から読む。
- 新しい自治体の追加は「マスタ登録 + 地区登録 + 収集設定 + 人口データ」で完結し、アプリ本体の改修が不要であること。
- 自治体 ID は総務省「全国地方公共団体コード」（6桁、TEXT）。URL には `slug` を使う（例: `/otawara`）。
- 自治体に属するデータを取得する API・クエリは必ず `municipality_id` で絞り込む。
- 収集処理は `Collector` trait で抽象化し、自治体ごとの差分はアダプタ + `collector_sources.config` に閉じ込める。
- Polis 分析（PCA / クラスタリング / コンセンサス判定）はトピック単位。自治体をまたいで混ぜない。

## 分断を生まない原則（`docs/adr/0002-participant-attributes.md`）
- **市民を本人の属性（年齢・性別など）や市内の地区で分けて集計・可視化しない。** 分けて見せてよいのは、その自治体との関わり方（`residency_type`: 住民・周辺・通勤通学・関係人口）だけ。
- 年齢・性別は集めない（必須は棄却、任意も当面しない）。参加者の地区も集めない。トピックの地区タグ（話題の対象地域）は人を分けないので可。
- 関わり方も、意見グループを作るのには使わず、グループの説明と意見ごとの比較にだけ使う（Polis の原則）。
- 公表は5人未満の区分を隠し、隠した合計が1〜4人なら次に少ない区分も隠す（二次秘匿。`backend/src/routes/analysis.rs` の `suppress`）。
- 属性や区分で集計する機能を検討するときは、プライバシーに加えて「分断を生まないか」を必ず検討する。

## 技術方針
- DB: 開発は SQLite、将来 PostgreSQL へ移行。
  - SQL は両方で動く書き方を優先（SQLite 固有関数を避ける）。
  - ID は TEXT（UUID v7 推奨。ただし行政区分マスタは公的コードを使用）。日時は ISO8601 文字列（UTC）。
  - マイグレーションは `backend/migrations/` に sqlx migrate 形式で置く。
  - main にマージ済みのマイグレーションは書き換えない（コメント1文字でも sqlx のチェックサムが変わり、既存の DB が起動時に弾かれる）。変更は新しいマイグレーションで行う。
- テーブル（定義は docs/grand-design.md §3）:
  - 行政区分: `prefectures` / `municipalities` / `districts` / `municipality_stats`
  - ユーザー: `users` / `user_municipality_relations`（居住地属性は自治体ごとにこちらで持つ）
  - Polis コア: `topics`（`municipality_id` 必須）/ `statements` / `votes`
  - タグ: `tags`（kind = `field` | `district`）/ `topic_tags`
  - 収集: `collector_sources`
- 制約:
  - `votes` は `(user_id, statement_id)` に UNIQUE。
  - `statements.status` は `pending` / `approved` / `rejected` / `flagged`。
  - `statements.body` は 140 文字以内（API 側でも検証）。
- 投稿者情報は API レスポンスに絶対に含めない（匿名性の担保）。
- 外部 API キー（LLM 等）は `.env` で管理し、コミットしない。`.env.example` を用意する。
- フロントは「コメント欄・リプライ」機能を作らない（Polis の原則）。
- フロントのルーティング: `/[municipality]` がトップ、`/[municipality]/topics/[id]` が詳細。
  - 有効な自治体が1つの間は選択画面を出さず、そのトップへ遷移する。

## 開発ルール
- 作業は小さな単位で進め、各ステップの終わりに動作確認 → コミットする。
- コミットメッセージは Conventional Commits（`feat:`, `fix:`, `chore:`, `docs:` 等）、本文は日本語可。
- 大きな変更の前には計画を提示し、承認を得てから実装する。
  - 承認された計画は、実装を始める前に `docs/plans/YYYY-MM-DD-<topic>.md` として保存し、最初にコミットする（あとから計画と実装を突き合わせて検証できるようにするため）。
  - 実装中に計画から外れた点は、完了時にその md の末尾に「実装時の変更点」として追記する。
- 新しい依存クレート / npm パッケージを追加する場合は理由を一言添える。
- テスト: backend は `cargo test`、frontend は lint と型チェックを通す。
  - 自治体の取り違えを防ぐため、テスト用 seed には最低2自治体を入れる。

## よく使うコマンド（整備され次第更新する）
- 初回: `cp .env.example .env`（`.env` はルートに1つ。backend / sqlx-cli は親ディレクトリを遡って読み、frontend は `next.config.ts` でルートの `.env` を読む）
  - `.env.example` に変数が増えたら手元の `.env` にも追記する（`NEXT_PUBLIC_SOURCE_URL` が無いと本番ビルドが失敗する）
  - `SESSION_SECRET`（参加者 Cookie の署名鍵）は必須。`openssl rand -hex 32` で作る。未設定・32 バイト未満なら backend は起動しない
  - `API_INTERNAL_URL`: Next.js のサーバー側から backend を呼ぶ URL（既定 `http://127.0.0.1:8080`）。ブラウザは同一オリジンの `/api` を呼び、開発時は `next.config.ts` の rewrites で backend に転送する
- backend（`cd backend`）
  - `cargo run` — 起動時にマイグレーションも自動適用。`GET /api/health` で DB 込みの疎通確認
  - `cargo run -- seed` — `data/seeds/*.sql` をファイル名順に投入して終了（冪等。初回とマスタ更新時に実行）
  - `cargo run -- seed --dev` — 続けて `data/dev-seeds/`（【サンプル】の架空データ）も投入する。開発用で、本番では使わない
    - 架空の参加者120人と投票も入るので、DB を初期化した直後から分析（意見マップ・グループ・合意点）を確認できる。`cargo run` で起動すると、ワーカーがすぐに分析を計算する
    - 投票データ `110_sample_votes_092100_otawara.sql` は `python3 scripts/generate_sample_votes.py > data/dev-seeds/110_sample_votes_092100_otawara.sql` で生成する（手で編集しない。シード固定で毎回同じ出力）
    - サンプルで分析が出ることは `cargo test --test seeds` で確かめている
  - `cargo run -- analyze [--topic <id>] [--force]` — 分析を今すぐ再計算する（通常は `ANALYSIS_INTERVAL_SECS`（既定 10 秒）ごとにバックグラウンドで、投票が変わったトピックだけ再計算される）
  - `cargo test` — インメモリ SQLite にマイグレーションを当ててテスト（`data/seeds/` の適用テストも含む）
  - `sqlx migrate add <name>` / `sqlx migrate run` / `sqlx migrate info`
  - `migrations/` の変更は `build.rs` により自動で再ビルドされる
- frontend（`cd frontend`）
  - `npm run dev` / `npm run lint` / `npm run typecheck` / `npm run build`
  - Next.js 16 系。書く前に `frontend/AGENTS.md` の指示どおり `node_modules/next/dist/docs/` を確認する
- API（すべて `/api` 配下。エラーは `{"error": {"code", "message"}}`）
  - `GET /municipalities` / `GET /municipalities/{slug}`（地区つき）
  - `GET /municipalities/{slug}/topics?tag=` / `GET /municipalities/{slug}/topics/{id}` — Cookie があれば参加状態 `participation`（`none` / `in_progress` / `completed`）と `voted_count` を返す（承認済みの意見だけで数える）
  - `PUT /municipalities/{slug}/relation` — このまちとの関わり方（投票の前に必要）。地区は受け取らない
  - `GET /municipalities/{slug}/topics/{id}/next-statement` — 未投票の承認済み意見1件と進捗（Polis の優先度で重み付けした抽選）
  - `GET /municipalities/{slug}/topics/{id}/analysis` — 意見マップ・グループと代表意見・合意点・`statements_by_group`・`group_composition`（グループごとの関わり方の構成）・関わり方ごとの集計（分析対象が5人未満なら insufficient。関わり方の集計は二次秘匿済み）
    - **そのトピックのすべての承認済み意見に投票するまでは `locked`（残り件数）を返す**。全体の傾向に投票が左右されないようにするため。意見が増えれば再びロックされる
  - `GET /municipalities/{slug}/consensus` — タイムラインにピン留めする合意点（全グループで (A+1)/(S+2) ≥ 0.8）。投票を終えたトピックの分だけを返す（Cookie が無ければ空）
  - `POST /municipalities/{slug}/topics/{id}/statements/{sid}/votes` — `{value: 1 | -1 | 0}`
  - `POST /municipalities/{slug}/topics/{id}/statements` — `{body}`（1〜140文字）
  - 参加者は署名付き HttpOnly Cookie `cp_uid` で識別（認証はフェーズ6）。CI は `.github/workflows/ci.yml`
- 自治体の追加: `data/seeds/010_municipality_<団体コード>_<slug>.sql` を1ファイル追加し、`cargo run -- seed`
  - 書式は `010_municipality_092100_otawara.sql` に倣う。全 INSERT に `ON CONFLICT DO NOTHING` を付ける
  - 地区 ID は `<団体コード>-<ローマ字>`。人口統計は出典 URL 付きで `municipality_stats` に入れる

## 開発フェーズ
1. 土台: モノレポ雛形、DB マイグレーション（行政区分マスタ含む）、大田原市の seed、ヘルスチェック API、フロント雛形
2. コア: 自治体別トピック一覧・詳細、ステートメント投稿、投票 API、Polis 風投票 UI
   - 開始時: CI（GitHub Actions）で PostgreSQL へのマイグレーション・seed 互換チェック
3. 分析: 投票行列からの PCA / クラスタリング、コンセンサス判定（全グループ賛成率 80% 以上）
4. AI: モデレーション、自動タグ付け（分野は共通語彙、地区はその自治体の districts から選択）
5. 収集: `Collector` trait と大田原市用アダプタ、決算カード共通パーサー（robots.txt と利用規約を確認してから）
   - 収集バッチは backend のサブコマンドとして作る（本番での定期実行はリリース準備で整える）
6. 認証: SNS ログイン（Google / Apple / LINE）→ 後に SMS 認証
7. リリース準備: 全機能がそろってから、デプロイ先の選定・契約、Dockerfile / 本番用 compose / Caddy / バックアップ、ドメイン、リポジトリの Public 化、本番公開をまとめて行う（`docs/adr/0001-docker-adoption.md` の改訂を参照）
8. 展開: 自治体選択 UI の有効化、近隣自治体の追加、自治体追加手順のドキュメント化