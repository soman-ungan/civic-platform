# CLAUDE.md — 地方行政可視化＆市民参加プラットフォーム（多自治体対応）

このファイルは Claude Code がこのリポジトリで作業する際の指針です。
docs の入口は `docs/README.md`。**設計判断で迷ったら、`docs/overview.md`（今の全体像）→ 該当する ADR（`docs/adr/`）→ `docs/design/schema.md`（テーブル）・`docs/design/api.md`（API）を読み直す。**
`docs/grand-design.md` は当初の構想（v2）の記録で、今の実装とは違うところがある。

## プロジェクト概要
- 自治体の議事録・予算・決算を可視化し、Polis の思想に基づく合意形成の場を提供する Web アプリ。
- **多自治体対応**: 行政区分（都道府県 → 市区町村 → 地区）を選択可能な構造。栃木県大田原市がパイロット自治体。
- 主対象は 10〜30 代。スマホファースト。
- 個人開発・ワンオペ運用。自動化と保守のしやすさを最優先する。
- Polis を丸ごとフォークはせず、データ構造・画面設計・数理ロジック（PCA + クラスタリング）を Rust × Next.js に移植する。
  - 数理・統計処理は Polis の正式なアルゴリズム（Clojure 実装）を移植したもの（`backend/src/process/analysis/`）。方針・Polis との違い・一致テストの結果は `docs/polis-math.md`。変更するときは `cargo test --test polis_parity` で Polis との一致を確かめる。

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
│   ├── src/shared/       システム内で共通に使うもの（config・db・limits（トピックと意見の文字数）・status（意見と公開の状態の型。DB の CHECK と同じ値）・util・llm）
│   ├── src/interface/    外とのやりとり。api/（HTTP API: 自治体・トピック・投票・分析・まとめ（トピックマップ）・議員の記録・まちのお金）と cli/（運営のコマンド。clap。<動詞> <対象>。docs/operations/cli.md）
│   ├── src/ingest/       段階1 資料を DB に入れる（collect: 取得と置き場・決算カード、budget、minutes、extraction）
│   ├── src/records/      段階2 DB を見る・直す（topics・budget・settlement・minutes・officials・statements・tags）
│   ├── src/process/      段階3 判定・計算（moderation・tagging・analysis（Polis の数理）・progress（参加のルール。ルールはここだけで変える）・similarity（似ているかの候補）・topic_map・digest（市民のまとめ））
│   └── src/maintenance/  準備と保守（seed・eval）
│       層の依存は interface → ingest / records / process / maintenance → shared。内部ロジックは interface を使わない（docs/plans/2026-10-28-backend-layers.md）
├── frontend/             Next.js (App Router) / TypeScript / Tailwind CSS
│   ├── src/components/   画面の部品。まとまりは backend の interface/api/ にそろえる（topics・participation・analysis・digest・officials・finances・auth）。ほかに ui/（よく使う見た目: チップ・横スクロールの行・切り替え・空の欄・説明の吹き出し・ボタンのクラス）と layout/（画面の枠）
│   └── src/lib/          api/（backend とのやりとり: paths・server・client・use-api。型は api/types/ に backend のモジュールごと）、finances/（お金の表示の決まり）、auth/（パスキー）、画面をまたぐ小さな決まり（labels など）
│       ファイル名はディレクトリ名と重なる部分を外す（components/finances/bars.tsx など）。型をまとめて export し直すファイルは作らない（docs/plans/2026-10-28-frontend-layout.md）
├── data/                 SQLite ファイル（*.db は git 管理しない）
│   └── seeds/            自治体・地区マスタの初期データ（git 管理する）
└── docs/                 入口は docs/README.md。overview（今の全体像）・design/（テーブル・API・収集）・operations/（運営者向け）・adr/・plans/
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
- 公表は5人未満の区分を隠し、隠した合計が1〜4人なら次に少ない区分も隠す（二次秘匿。`backend/src/process/analysis/disclosure.rs` の `suppress`）。
- 属性や区分で集計する機能を検討するときは、プライバシーに加えて「分断を生まないか」を必ず検討する。

## AI（LLM）の原則（`docs/adr/0003-llm-operations.md`）
- Claude API は `backend/src/shared/llm/` から reqwest で直接呼ぶ（Rust に公式 SDK が無いため）。応答は構造化出力で受け取る。
- **API キーが無くても動くこと**（モデレーションは全件承認（公人の氏名の確かめだけは効く）、似た意見の判定とタグ付けはしない）。テストは偽の LLM（`Llm::fake`）で行い、実際の API は呼ばない。実際の API を呼ぶ評価（`eval`）は費用がかかるので、実行の前にユーザーの了承を得る。
- 意見の審査（ADR 0007）は主張や政治的な立場では判断しない。1回の呼び出しで、害・公序良俗／人物／投票できる意見か・トピックとの関係／似た意見 を別々に答えさせる。公人の政策・公務への批判は公開、人格攻撃・私生活・人物の評価や進退（ほめても同じ）は非公開。投票できない文・無関係な文は非公開（迷ったら公開）。公人の氏名は AI の前にプログラムで確かめる
- **判定できないとき（通信の失敗・費用の上限）は pending のまま残し、自動承認しない。** refusal は flagged（人が確認）。
- AI に送るのは本文と、公開済みのトピックの内容（審査ではタイトル・説明・承認済みの意見の本文、タグ付けはタイトルと説明）だけ。参加者 ID・関わり方・トピックや意見の ID は送らない。
- 首長・議員などの人物はタグにしない。議員ごとの記録（タブ「議会」。ADR 0008）は、会議録の発言者の記載から機械的に対応付けたもの（`topic_sources.official_id`）だけを見せる（LLM に推測させない。順位・評価・立場の推測・市民の投票の結果の議員ごとの集計はしない）。投票の対象の意見の文には議員名を入れない。
- 行政資料の収集は「エージェント」にせず、設定に従う決まった手順のコードにする。原本は保持しない（`docs/design/collection.md`）。

## 会議録の原則（`docs/adr/0004-assembly-minutes.md`）
- 名前を出すのは選挙で選ばれた公人（首長・議員）だけ。`public_officials` と会議録の発言者の記載から機械的に対応付ける。職員は職名だけ（AI にも氏名を渡さない）。
- 引用は会議録の原文にそのまま含まれるものだけ。説明と意見に氏名を入れない（プログラムで確かめる）。
- 最初の意見は議会で実際に出た主張だけを出発点にする（根拠の発言を `statement_sources` に記録。API では返さない）。AI に新しい選択肢を作らせない。
- AI が作ったトピックは下書き（`topics.status = 'draft'`）。運営者が確かめてから公開する。下書きは API のどこにも出さない。
- 手でコピーした会議録は `data/private/`（git の管理外）に置く。リポジトリに入れない。

## トピックの寿命・投稿数・資金の原則（`docs/adr/0005-topic-lifecycle-and-funding.md`）
- トピックは期限つき（既定は公開から90日。運営者が早めに締め切れる）。締め切っても結果は読める。止めるのは投票と投稿だけ
- 意見は、参加する人は1トピック1件まで（承認待ちを含む）。市民提案のトピックでは、提案した本人だけ3件まで（提案に含めた最初の意見1〜10件とは別に）。議会のトピックの最初の意見は、議会で出た主張をすべて採る（数で切らない）。市民からのトピックは提案として受け、運営者の確認を通ったものだけを公開する（1人が1つの自治体に週1件まで。提案した人 `topics.proposed_by` はどの API にも返さない）
- 似た意見（同じ立場で同じこと）は、AI が見つけたら公開せず、既存の意見への投票を案内する（上限に数えない）。立場が違えば別の意見として通す
- 分析は、最初の意見すべてに投票したら見られる（市民の意見は数えないので、公開後に一度開いた分析は閉じない）。次の意見は最初の意見から出す。意見の投稿も、分析を見られるようになってから（`backend/src/process/progress.rs`）。投稿した人は自分の意見に賛成したものとする（Polis と同じ。市民の提案の最初の意見は、Polis の seed と同じく自動の票なし）
- **お金（寄付・投げ銭）で議論への影響を変えない。** 投稿数・投票・表示の順番は誰でも同じ。行政からの出資と、政党・政治団体・候補者からの寄付は受けない。収支は公開する
- 会議録から作るトピックが既存と同じ論点（何を決める・変えるかが同じ）なら、新しく作らず意見を足す（下書きには最初の意見として、公開中には確認待ちで。`statements.is_seed`）。意見が少ないトピックも残して運営者が確かめる。AI に反対の立場の意見を作らせない。似ているかは、プログラムが候補を絞り（文字の2-gram の類似度。`backend/src/process/similarity.rs`。トピックと意見に共通）、AI か運営者が決める
- トピックの投票は混ぜない。統合は「市民のまとめ」で行う: 市民が合意点のページ（`/{slug}/consensus`）のトピックマップ（締め切ったトピックを似ている度合いで配置。タグで絞る）で起点と近傍を選び、合意点・意見が分かれた点・グループの代表的な意見を原文と数字のまま見て、自分で次のトピックを提案する（普通の提案と同じく週1件・運営者の確認。元のトピックは `topic_parents`）
  - 中身はプログラムだけで決める（`backend/src/process/digest.rs`）。AI で論点をまとめない。グループを AI で要約しない（人をひとくくりに名付けない）

## 技術方針
- DB: 開発は SQLite、将来 PostgreSQL へ移行。
  - SQL は両方で動く書き方を優先（SQLite 固有関数を避ける）。
  - ID は TEXT（UUID v7 推奨。ただし行政区分マスタは公的コードを使用）。日時は ISO8601 文字列（UTC）。
  - マイグレーションは `backend/migrations/` に sqlx migrate 形式で置く。
  - main にマージ済みのマイグレーションは書き換えない（コメント1文字でも sqlx のチェックサムが変わり、既存の DB が起動時に弾かれる）。変更は新しいマイグレーションで行う。
- テーブルの一覧と中身は `docs/design/schema.md`（正はマイグレーション）。テーブルを足したり変えたりしたら、そこと `docs/operations/data-flow.md` の「テーブルの一覧」を直す
  - 状態の値（`statements.status`・`topics.status` など）は DB の CHECK と `backend/src/shared/status.rs` の型が同じ（`backend/tests/it/shared/schema.rs`）
  - `votes` は `(user_id, statement_id)` に UNIQUE。`statements.body` は 140 文字以内（API 側でも検証）
- 投稿者情報は API レスポンスに絶対に含めない（匿名性の担保。運営者の API でも）。下書きは運営者の API 以外のどこにも出さない。
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
- docs の置き場所（`docs/README.md`）: 運営者向けは `docs/operations/`、開発者向けの今の設計は `docs/design/` と `docs/overview.md`、決めたことは ADR、計画は `docs/plans/`（記録なので、あとから中身を直さない。索引は `docs/plans/README.md`）
  - マイグレーションのコメントが参照している docs のファイル（`grand-design.md`・`polis-math.md`・`design/collection.md`・`design/tagging-criteria.md`・`adr/`・`plans/`）は動かさない
- テスト: backend は `cargo test`、frontend は lint と型チェックを通す。
  - 自治体の取り違えを防ぐため、テスト用 seed には最低2自治体を入れる。

## よく使うコマンド
- 運営のコマンドの全体は `docs/operations/cli.md`、資料・コマンド・DB のデータの流れは `docs/operations/data-flow.md`。コマンドやテーブルを足したり変えたりしたら、そちらも直す
- 初回: `cp .env.example .env`（`.env` はルートに1つ。backend / sqlx-cli は親ディレクトリを遡って読み、frontend は `next.config.ts` でルートの `.env` を読む）
  - `.env.example` に変数が増えたら手元の `.env` にも追記する（`NEXT_PUBLIC_SOURCE_URL` が無いと本番ビルドが失敗する）
  - `SESSION_SECRET`（参加者 Cookie の署名鍵）は必須。`openssl rand -hex 32` で作る。未設定・32 バイト未満なら backend は起動しない
  - `API_INTERNAL_URL`: Next.js のサーバー側から backend を呼ぶ URL（既定 `http://127.0.0.1:8080`）。ブラウザは同一オリジンの `/api` を呼び、開発時は `next.config.ts` の rewrites で backend に転送する
- backend（`cd backend`）
  - `cargo run` — API サーバー（起動時にマイグレーションを自動で適用。`GET /api/health`）。分析・判定待ちの再判定・自動のタグ付けがバックグラウンドで動く
  - `cargo run -- seed [--dev]` — `data/seeds/*.sql`（冪等）。`--dev` は `data/dev-seeds/`（【サンプル】の架空データ。参加者120人と投票つきで、すぐ分析を確かめられる。本番では使わない）
    - 投票データ `110_sample_votes_092100_otawara.sql` は `python3 scripts/generate_sample_votes.py > data/dev-seeds/110_sample_votes_092100_otawara.sql` で生成する（手で編集しない）。サンプルで分析が出ることは `cargo test --test it maintenance::seeds`
  - **運営のコマンドは `cargo run -- <動詞> <対象> [オプション]`**（`docs/operations/cli.md`）。動詞は段階ごと: 資料を DB に入れる `fetch`/`extract`/`import`、DB を見る・直す `list`/`show`/`edit`/`add`/`remove`/`publish`/`archive`、判定・計算 `moderate`/`classify`/`analyze`
    - コマンドを足すときは、段階を決めてから `src/interface/cli/` の `ingest.rs`・`records/`（動詞ごとに `list`・`show`・`edit`・`publish`）・`process.rs` のどれかに置き、中身（SQL・処理）は同じ段階の `src/{ingest,records,process}/` に置く。その対象に要らない処理は「何もしません」で終了コード 0
  - **費用がかかるのは AI を呼ぶコマンド**（`extract`（`--manual`・`--dry-run` を除く）・`moderate`・`classify`・`eval`）。`eval` はユーザーの了承を得てから。資料の取り込みは API を使わない共同作業（`--manual`。`.claude/commands/import-minutes.md`・`import-budget.md`）でもできる
  - 確認待ちの意見の採否・下書きの公開・期限の変更は、管理画面（`/operator`）でも同じことができる（中身は同じ処理）
  - `cargo test` — インメモリ SQLite にマイグレーションを当ててテスト（`data/seeds/` の適用テストも含む）。統合テストは `tests/it/`（1つのバイナリ。まとまりは `src/` の階層にそろえる。絞るなら `cargo test --test it api::operator` のようにモジュールの名前で）、ほかに `tests/cli.rs`（実際のバイナリ）と `tests/polis_parity.rs`
  - `sqlx migrate add <name>` / `sqlx migrate run` / `sqlx migrate info`。`migrations/` の変更は `build.rs` により自動で再ビルドされる
- frontend（`cd frontend`）
  - `npm run dev` / `npm run lint` / `npm run typecheck` / `npm run build`
  - Next.js 16 系。書く前に `frontend/AGENTS.md` の指示どおり `node_modules/next/dist/docs/` を確認する
  - `globals.css` を変えても開発サーバーが古い CSS を配り続けることがある（Turbopack のキャッシュ。WSL で確認）。そのときは `npm run dev` を止めて `.next/dev` を消してから起動し直す
- API は `docs/design/api.md`（すべて `/api` 配下。エラーは `{"error": {"code", "message"}}`）。API を足したり変えたりしたら、そこを直す
  - 分析を見られるか・投稿できるかのルールは `backend/src/process/progress.rs` の1か所だけ。画面は API の `progress`・`posting` を見る（ルールを写さない）
  - 参加者は署名付き HttpOnly Cookie `cp_uid`。読む・投票はログインなし、投稿はパスキーの登録が要る（ADR 0006）
  - 運営者の API（`/api/operator/...`）は運営者でなければすべて 404。中身は CLI と同じ `records`・`process` の関数（ロジックを写さない）
- 自治体の追加: `data/seeds/010_municipality_<団体コード>_<slug>.sql` を1ファイル追加し、`cargo run -- seed`
  - 書式は `010_municipality_092100_otawara.sql` に倣う。全 INSERT に `ON CONFLICT DO NOTHING` を付ける
  - 地区 ID は `<団体コード>-<ローマ字>`
  - 決算カードの収集先（`soumu_settlement_card`）を1行足して `fetch settlement -m <slug>` → `import settlement -m <slug>` すると、人口（`municipality_stats`）も決算カードから入る。設定は全市町村で同じ（大田原市の行をそのまま写し、id と municipality_id だけ変える）
- CI は `.github/workflows/ci.yml`（PostgreSQL へのマイグレーションと seed の互換も確かめる）

## 開発フェーズ
- 1 土台 → 2 コア → 3 分析 → 4 AI → 5 収集 → 6a パスキー・運営者 → 6b 参加の決まり → 6c 管理画面とトピックの期限 → 6d 市民からのトピックの提案 → タイムラインの流れ → 意見の取りまとめ（類似判定・トピックマップ・市民のまとめ） は済み（状況は `docs/overview.md`、各計画は `docs/plans/README.md`）
- 6e 1人を見分ける目印（乱用が見えたら。SNS か SMS を1回だけ確かめ、鍵付きのハッシュだけを持つ。ADR 0006）
- 7 リリース準備: 全機能がそろってから、デプロイ先の選定・契約、Dockerfile / 本番用 compose / Caddy / バックアップ、ドメイン、リポジトリの Public 化、本番公開をまとめて行う（`docs/adr/0001-docker-adoption.md` の改訂を参照）。収集の定期実行もここで整える
- 8 展開: 自治体選択 UI の有効化、近隣自治体の追加、自治体追加手順のドキュメント化
