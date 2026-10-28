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
├── backend/              Rust / axum / sqlx（API サーバー＋運営のコマンド）
│   ├── src/routes/       API（自治体・トピック・投票・分析・まちのお金）
│   ├── src/progress.rs   参加のルール（進み具合・分析を見られるか・参加できるトピックか）。ルールはここだけで変える
│   ├── src/status.rs     意見と公開の状態の型（DB の CHECK と同じ値）
│   ├── src/cli/          運営のコマンド（clap。docs/cli.md）
│   ├── src/collect/      行政資料の収集アダプタ
│   └── src/{analysis,moderation,tagging,minutes,budget,llm}/  分析・AI・取り込み
├── frontend/             Next.js (App Router) / TypeScript / Tailwind CSS
│   ├── src/components/ui/  よく使う見た目（チップ・横スクロールの行・切り替え・空の欄・ボタンのクラス）
│   └── src/lib/          API（api-paths・api-server・api-client・use-api）、型（types.ts）、表示の決まり
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

## AI（LLM）の原則（`docs/adr/0003-llm-operations.md`）
- Claude API は `backend/src/llm/` から reqwest で直接呼ぶ（Rust に公式 SDK が無いため）。応答は構造化出力で受け取る。
- **API キーが無くても動くこと**（モデレーションは全件承認、タグ付けはしない）。テストは偽の LLM（`Llm::fake`）で行い、実際の API は呼ばない。実際の API を呼ぶ評価（`eval`）は費用がかかるので、実行の前にユーザーの了承を得る。
- モデレーションは主張や政治的な立場では判断しない。公人の公務への批判は公開、人格攻撃・私生活は非公開。
- **判定できないとき（通信の失敗・費用の上限）は pending のまま残し、自動承認しない。** refusal は flagged（人が確認）。
- AI に送るのは本文（タグ付けはトピックのタイトルと説明）だけ。参加者 ID・関わり方・トピック ID は送らない。
- 首長・議員などの人物はタグにしない。議員ごとの記録はフェーズ5で、会議録の発言者の記載から機械的に対応付ける（LLM に推測させない）。投票の対象の意見の文には議員名を入れない。
- 行政資料の収集は「エージェント」にせず、設定に従う決まった手順のコードにする。原本は保持しない（`docs/design/collection.md`）。

## 会議録の原則（`docs/adr/0004-assembly-minutes.md`）
- 名前を出すのは選挙で選ばれた公人（首長・議員）だけ。`public_officials` と会議録の発言者の記載から機械的に対応付ける。職員は職名だけ（AI にも氏名を渡さない）。
- 引用は会議録の原文にそのまま含まれるものだけ。説明と意見に氏名を入れない（プログラムで確かめる）。
- 最初の意見は議会で実際に出た主張だけを出発点にする（根拠の発言を `statement_sources` に記録。API では返さない）。AI に新しい選択肢を作らせない。
- AI が作ったトピックは下書き（`topics.status = 'draft'`）。運営者が確かめてから公開する。下書きは API のどこにも出さない。
- 手でコピーした会議録は `data/private/`（git の管理外）に置く。リポジトリに入れない。

## トピックの寿命・投稿数・資金の原則（`docs/adr/0005-topic-lifecycle-and-funding.md`）
- トピックは期限つき（既定は公開から90日。運営者が早めに締め切れる）。締め切っても結果は読める。止めるのは投票と投稿だけ
- 意見は、参加する人は1トピック1件まで（承認待ちを含む）。市民提案のトピックでは、提案した本人だけ3件まで。市民からのトピックは提案として受け、運営者の確認を通ったものだけを公開する（1人週1件まで）
- 似た意見（同じ立場で同じこと）は、AI が見つけたら公開せず、既存の意見への投票を案内する（上限に数えない）。立場が違えば別の意見として通す
- 分析は、最初の意見すべてと合わせて一定の数（既定15件）に投票したら見られる（いまの実装は「全部に投票するまで」。フェーズ6で変える）
- **お金（寄付・投げ銭）で議論への影響を変えない。** 投稿数・投票・表示の順番は誰でも同じ。行政からの出資と、政党・政治団体・候補者からの寄付は受けない。収支は公開する
- トピックの投票は混ぜない。統合は「提言」（合意点と意見が分かれた点を原文と数字のまま集める）と、それを元にした「第2回のトピック」で行う（提言の作り方は検討中）

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
- 運営のコマンドの全体（利用する人向け）は `docs/cli.md`。コマンドを足したり変えたりしたら、そちらも直す
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
  - AI（`ANTHROPIC_API_KEY` があるときだけ。モデルは `MODERATION_MODEL` / `TAGGING_MODEL`、1日の費用の上限は `LLM_DAILY_BUDGET_USD`。使用量と推定費用は `llm_usage`）
    - `cargo run -- moderation list | approve <id> | reject <id> | retry` — 確認待ち（flagged / pending）の意見を扱う（認証のフェーズ6までは CLI）
    - `cargo run -- tag [--topic <id>] [--force]` — 自動タグ付け（通常は60秒ごとにバックグラウンドで、タグの無いトピックを処理）。`--force` は AI のタグだけを付け直す
    - `cargo run -- eval moderation|tagging [--model <id>] [--limit <n>]` — `evals/*.jsonl` で評価する。**実際の API を呼ぶので費用がかかる**
    - 分野タグの共通語彙は `data/seeds/020_field_tags.sql`（定義は `tags.description`、判断基準は `docs/design/tagging-criteria.md`）
  - 会議録（`EXTRACTION_MODEL`、既定 Opus 5.5。抽出の1日の上限は `LLM_EXTRACTION_DAILY_BUDGET_USD` で別枠）
    - `cargo run -- import minutes --municipality <slug> --file <path> --title <会議名> --held-on <YYYY-MM-DD> [--url <URL>] [--dry-run] [--again] [--force]` — 会議録からトピックの下書きを作る。`--dry-run` は AI を呼ばずに発言者・名簿に無い人・議事日程を出す。一般質問・代表質問の無い会議録は飛ばす（`--force` で取り込む）。**実際の API を呼ぶので費用がかかる**（1日分でおよそ 0.3〜0.4 ドル）
    - `cargo run -- topics drafts | show <id> | publish <id> | archive <id>` — 下書きの確認と公開
    - `cargo run -- topics edit <id> [--title] [--description]` / `topics statement add|edit|remove ...` / `topics tag add|remove <topic_id> <tag_id>` — 手直し
    - 公人の名簿は `data/seeds/030_officials_<団体コード>_<slug>.sql`
  - 収集（`docs/design/collection.md`。User-Agent に入れる連絡先 `COLLECTOR_CONTACT` が必須）
    - `cargo run -- collect list` — 収集先・規約の確認日・最後の取得
    - `cargo run -- collect run [--source <id>] [--year <n>] [--dry-run] [--ai]` — 有効な収集先から取得する（総務省の決算カード → `settlement_items` と `municipality_stats`）。取得はすべて `collect::fetch::Fetcher` を通す（robots.txt・間隔・条件付き取得）。規約の確認の記録（`terms_url`・`terms_checked_at`）が無い収集先は動かない。AI で資料を読む収集先（当初予算の概要）は、`--source` で選ぶか `--ai` を付けたときだけ動く（**費用がかかる**）
  - 予算（当初予算の概要の PDF。`docs/plans/2026-10-27-phase5c-budget.md`。`EXTRACTION_MODEL` と抽出の1日の上限を使う。**poppler-utils（`pdftotext`）が要る**）
    - `cargo run -- import budget --municipality <slug> --file <pdf> --year <n> [--url <URL>] [--dry-run] [--again]` — 手で取った PDF から下書きを作る。**実際の API を呼ぶので費用がかかる**（40ページで約 0.8 ドル）。`--dry-run` は読んで確かめた結果を出すだけ
    - `cargo run -- budget list | show <slug> <year> | publish <slug> <year> | archive <slug> <year>` — 下書きの確認と公開。下書きは API のどこにも出ない
    - 款別・性質別・歳入は合計で検算し、事業は名前と金額がそのページの本文にあるものだけを残す。事業の説明の文章は持たない（市の著作物のため。原本のページへリンクする）
    - 費目と分野タグの対応は `data/seeds/025_settlement_item_tags.sql`（おおまか。画面でも注記する）
  - `cargo test` — インメモリ SQLite にマイグレーションを当ててテスト（`data/seeds/` の適用テストも含む）
  - `sqlx migrate add <name>` / `sqlx migrate run` / `sqlx migrate info`
  - `migrations/` の変更は `build.rs` により自動で再ビルドされる
- frontend（`cd frontend`）
  - `npm run dev` / `npm run lint` / `npm run typecheck` / `npm run build`
  - Next.js 16 系。書く前に `frontend/AGENTS.md` の指示どおり `node_modules/next/dist/docs/` を確認する
  - `globals.css` を変えても開発サーバーが古い CSS を配り続けることがある（Turbopack のキャッシュ。WSL で確認）。そのときは `npm run dev` を止めて `.next/dev` を消してから起動し直す
- API（すべて `/api` 配下。エラーは `{"error": {"code", "message"}}`）
  - `GET /municipalities` / `GET /municipalities/{slug}`（地区つき）
  - `GET /municipalities/{slug}/topics?tag=` / `GET /municipalities/{slug}/topics/{id}` — Cookie があれば参加状態 `participation`（`none` / `in_progress` / `completed`）と `voted_count` を返す（承認済みの意見だけで数える）
  - `PUT /municipalities/{slug}/relation` — このまちとの関わり方（投票の前に必要）。地区は受け取らない
  - `GET /municipalities/{slug}/topics/{id}/next-statement` — 未投票の承認済み意見1件と進捗 `progress`（`voted`・`total`・`analysis_unlocked`。Polis の優先度で重み付けした抽選）
  - `GET /municipalities/{slug}/topics/{id}/analysis` — 意見マップ・グループと代表意見・合意点・`statements_by_group`・`group_composition`（グループごとの関わり方の構成）・関わり方ごとの集計（分析対象が5人未満なら insufficient。関わり方の集計は二次秘匿済み）
    - **そのトピックのすべての承認済み意見に投票するまでは `locked`（残り件数）を返す**。全体の傾向に投票が左右されないようにするため。意見が増えれば再びロックされる
    - 分析を見られるかのルールは `backend/src/progress.rs` の `Progress::analysis_unlocked` の1か所だけ。画面は `progress.analysis_unlocked` を見る（ルールを写さない）
  - `GET /municipalities/{slug}/finances?year=` — まちのお金。年度ごとに `settlement`（決算カード）と `budget`（公開済みの当初予算。主な事業つき）。金額は千円、`per_capita` は円。年度を省くと最新（予算だけの年度を含む）。`trend` は決算の年度と、決算がまだ無く予算だけがある年度（`kind`: settlement / budget）。トピックの詳細の `finances` は、分野タグに対応する目的別歳出と、今年度の主な事業（5件まで）
  - `GET /municipalities/{slug}/consensus` — タイムラインにピン留めする合意点（全グループで (A+1)/(S+2) ≥ 0.8）。投票を終えたトピックの分だけを返す（Cookie が無ければ空）
  - `POST /municipalities/{slug}/topics/{id}/statements/{sid}/votes` — `{value: 1 | -1 | 0}`
  - `POST /municipalities/{slug}/topics/{id}/statements` — `{body}`（1〜140文字）。判定の結果 `status`（approved / rejected / flagged / pending）と、rejected のときは言い換えの提案 `suggestion` を返す
  - 参加者は署名付き HttpOnly Cookie `cp_uid` で識別（認証はフェーズ6）。CI は `.github/workflows/ci.yml`
- 自治体の追加: `data/seeds/010_municipality_<団体コード>_<slug>.sql` を1ファイル追加し、`cargo run -- seed`
  - 書式は `010_municipality_092100_otawara.sql` に倣う。全 INSERT に `ON CONFLICT DO NOTHING` を付ける
  - 地区 ID は `<団体コード>-<ローマ字>`
  - 決算カードの収集先（`soumu_settlement_card`）を1行足して `collect run` すると、人口（`municipality_stats`）も決算カードから入る。設定は全市町村で同じ（大田原市の行をそのまま写し、id と municipality_id だけ変える）

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