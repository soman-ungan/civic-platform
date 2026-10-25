# CLAUDE.md — 地方行政可視化＆市民参加プラットフォーム（多自治体対応）

このファイルは Claude Code がこのリポジトリで作業する際の指針です。
詳細な設計は `docs/grand-design.md` を参照すること（設計判断で迷ったら必ず読み直す）。

## プロジェクト概要
- 自治体の議事録・予算・決算を可視化し、Polis の思想に基づく合意形成の場を提供する Web アプリ。
- **多自治体対応**: 行政区分（都道府県 → 市区町村 → 地区）を選択可能な構造。栃木県大田原市がパイロット自治体。
- 主対象は 10〜30 代。スマホファースト。
- 個人開発・ワンオペ運用。自動化と保守のしやすさを最優先する。
- Polis を丸ごとフォークはせず、データ構造・画面設計・数理ロジック（PCA + クラスタリング）を Rust × Next.js に移植する。

## ライセンス（AGPL-3.0-or-later）
- 本プロジェクトは「行政の透明性」を掲げるため、サービス自体も透明であることを最優先し、**GNU AGPL v3（AGPL-3.0-or-later）** とする。
- ルートに `LICENSE`（AGPLv3 全文）を置く。`backend/Cargo.toml` は `license = "AGPL-3.0-or-later"`、`frontend/package.json` は `"license": "AGPL-3.0-or-later"` とする。
- 新規ソースファイルの先頭に SPDX ヘッダーを付ける: `// SPDX-License-Identifier: AGPL-3.0-or-later`（SQL は `--`）。
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

## 技術方針
- DB: 開発は SQLite、将来 PostgreSQL へ移行。
  - SQL は両方で動く書き方を優先（SQLite 固有関数を避ける）。
  - ID は TEXT（UUID v7 推奨。ただし行政区分マスタは公的コードを使用）。日時は ISO8601 文字列（UTC）。
  - マイグレーションは `backend/migrations/` に sqlx migrate 形式で置く。
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
- 新しい依存クレート / npm パッケージを追加する場合は理由を一言添える。
- テスト: backend は `cargo test`、frontend は lint と型チェックを通す。
  - 自治体の取り違えを防ぐため、テスト用 seed には最低2自治体を入れる。

## よく使うコマンド（整備され次第更新する）
- 初回: `cp .env.example .env`（`.env` はルートに1つ。backend / sqlx-cli は親ディレクトリを遡って読み、frontend は `next.config.ts` でルートの `.env` を読む）
  - `.env.example` に変数が増えたら手元の `.env` にも追記する（`NEXT_PUBLIC_SOURCE_URL` が無いと本番ビルドが失敗する）
- backend（`cd backend`）
  - `cargo run` — 起動時にマイグレーションも自動適用。`GET /health` で DB 込みの疎通確認
  - `cargo run -- seed` — `data/seeds/*.sql` をファイル名順に投入して終了（冪等。初回とマスタ更新時に実行）
  - `cargo test` — インメモリ SQLite にマイグレーションを当ててテスト（`data/seeds/` の適用テストも含む）
  - `sqlx migrate add <name>` / `sqlx migrate run` / `sqlx migrate info`
  - `migrations/` の変更は `build.rs` により自動で再ビルドされる
- frontend（`cd frontend`）
  - `npm run dev` / `npm run lint` / `npm run typecheck` / `npm run build`
  - Next.js 16 系。書く前に `frontend/AGENTS.md` の指示どおり `node_modules/next/dist/docs/` を確認する
- 自治体の追加: `data/seeds/010_municipality_<団体コード>_<slug>.sql` を1ファイル追加し、`cargo run -- seed`
  - 書式は `010_municipality_092100_otawara.sql` に倣う。全 INSERT に `ON CONFLICT DO NOTHING` を付ける
  - 地区 ID は `<団体コード>-<ローマ字>`。人口統計は出典 URL 付きで `municipality_stats` に入れる

## 開発フェーズ
1. 土台: モノレポ雛形、DB マイグレーション（行政区分マスタ含む）、大田原市の seed、ヘルスチェック API、フロント雛形
2. コア: 自治体別トピック一覧・詳細、ステートメント投稿、投票 API、Polis 風投票 UI
3. 分析: 投票行列からの PCA / クラスタリング、コンセンサス判定（全グループ賛成率 80% 以上）
4. AI: モデレーション、自動タグ付け（分野は共通語彙、地区はその自治体の districts から選択）
5. 収集: `Collector` trait と大田原市用アダプタ、決算カード共通パーサー（robots.txt と利用規約を確認してから）
6. 認証: SNS ログイン（Google / Apple / LINE）→ 後に SMS 認証
7. 展開: 自治体選択 UI の有効化、近隣自治体の追加、自治体追加手順のドキュメント化