# CLAUDE.md — 大田原市・行政可視化＆市民参加プラットフォーム

このファイルは Claude Code がこのリポジトリで作業する際の指針です。
詳細な設計は `docs/grand-design.md` を参照すること（設計判断で迷ったら必ず読み直す）。

## プロジェクト概要
- 栃木県大田原市の議事録・予算・決算を可視化し、Polis の思想に基づく合意形成の場を提供する Web アプリ。
- 主対象は 10〜30 代。スマホファースト。
- 個人開発・ワンオペ運用。自動化と保守のしやすさを最優先する。
- Polis のコードはフォークしない。データ構造・画面設計・数理ロジック（PCA + クラスタリング）の考え方のみを独自実装する。
  - 注意: Polis 本家は AGPL-3.0。コードの直接コピーは禁止（独自実装のみ）。

## リポジトリ構成（モノレポ）
```
otawara-civic-platform/
├── backend/    Rust / axum / sqlx（API サーバー）
├── frontend/   Next.js (App Router) / TypeScript / Tailwind CSS
├── data/       SQLite ファイル（git 管理しない）
└── docs/       設計書・ADR
```

## 技術方針
- DB: 開発は SQLite、将来 PostgreSQL へ移行。
  - SQL は両方で動く書き方を優先（SQLite 固有関数を避ける）。
  - ID は TEXT（UUID v7 推奨）。日時は ISO8601 文字列（UTC）。
  - マイグレーションは `backend/migrations/` に sqlx migrate 形式で置く。
- コアテーブル: `users` / `topics` / `statements` / `votes`（定義は docs/grand-design.md §3）。
  - `votes` は `(user_id, statement_id)` に UNIQUE 制約。
  - `statements.status` は `pending` / `approved` / `rejected` / `flagged`。
  - `statements.body` は 140 文字以内（API 側でも検証）。
- 投稿者情報は API レスポンスに絶対に含めない（匿名性の担保）。
- 外部 API キー（LLM 等）は `.env` で管理し、コミットしない。`.env.example` を用意する。
- フロントは「コメント欄・リプライ」機能を作らない（Polis の原則）。

## 開発ルール
- 作業は小さな単位で進め、各ステップの終わりに動作確認 → コミットする。
- コミットメッセージは Conventional Commits（`feat:`, `fix:`, `chore:`, `docs:` 等）、本文は日本語可。
- 大きな変更の前には計画を提示し、承認を得てから実装する。
- 新しい依存クレート / npm パッケージを追加する場合は理由を一言添える。
- テスト: backend は `cargo test`、frontend は lint と型チェックを通す。

## よく使うコマンド（整備され次第更新する）
- 初回: `cp .env.example .env`（`.env` はルートに1つ。backend / sqlx-cli は親ディレクトリを遡って読む）
- backend（`cd backend`）
  - `cargo run` — 起動時にマイグレーションも自動適用。`GET /health` で DB 込みの疎通確認
  - `cargo test` — インメモリ SQLite にマイグレーションを当ててテスト
  - `sqlx migrate add <name>` / `sqlx migrate run` / `sqlx migrate info`
  - `migrations/` の変更は `build.rs` により自動で再ビルドされる
- frontend（`cd frontend`）
  - `npm run dev` / `npm run lint` / `npm run typecheck` / `npm run build`
  - Next.js 16 系。書く前に `frontend/AGENTS.md` の指示どおり `node_modules/next/dist/docs/` を確認する

## 開発フェーズ
1. 土台: モノレポ雛形、DB マイグレーション、ヘルスチェック API、フロント雛形
2. コア: トピック一覧・詳細、ステートメント投稿、投票 API、Polis 風投票 UI
3. 分析: 投票行列からの PCA / クラスタリング、コンセンサス判定（全グループ賛成率 80% 以上）
4. AI: モデレーション、自動タグ付け
5. 収集: 大田原市公式 HP の議事録・決算カードのクローラー（robots.txt と利用規約を確認してから）
6. 認証: SNS ログイン（Google / Apple / LINE）→ 後に SMS 認証