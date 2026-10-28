# フェーズ6の前のコードの整理（見た目と動きは変えない）

## Context
フェーズ5までで機能を足し続けたので、フェーズ6（認証）と ADR 0005 の機能（意見の上限・似た意見の知らせ・分析を見られるまでの投票数・市民の提案・トピックの締め切り）を入れる前に、コードを整理する。
調べて分かったこと（backend・frontend を調査）:
- **参加のルールが散らばっている**: 進み具合（`progress`）・参加の状態・分析のロック（全部に投票したら開く）・トピックが投票できる状態かの確認が、`routes/topics.rs`・`routes/participation.rs`・`routes/analysis.rs` にばらばらにあり、同じ SQL の写しもある（`next_statement` が `progress` を写している、`vote` が独自にトピックの状態を確かめる、`RESIDENCY_TYPES` が2か所）。ADR 0005 の「最初の意見すべて＋15件で開く」は、backend の2か所と frontend（`topic-interaction.tsx` の `voted >= total`、`analysis-panel.tsx` の文言）を同時に直さないといけない
- **状態の文字列**（`'approved'`・`'published'` など）が SQL に直書き。フェーズ6で状態が増える（提案・締め切り）
- **CLI**: `main.rs`（677行）が引数を手で読み、同じ処理の写しが多い
- **frontend**: API の URL を7か所で組み立て、取得の処理（読み込み・エラー・やり直し）が投票と分析で二重。トピックの画面は Cookie を backend に渡していない（認証後の情報が取れない）。チップ・切り替え・バッジ・空の表示・横スクロールの行のクラスの写しが3〜7か所。UI でない関数が部品のファイルにある（`fieldGroups`・`tabsFor`）

**方針**: 画面の見た目・API の応答・CLI の使い方は変えない（例外は B4 の `analysis_unlocked` の追加だけ）。フェーズ6でしか決まらないもの（users の作り直し・認証の種類・管理画面・`Unauthorized` のエラー）は今回は作らない。users の使っていない列（年齢・性別・district_id など）は、フェーズ6で users を作り直すときにまとめて消す。

## 進め方
- ブランチ `refactor/pre-phase6`。計画を `docs/plans/2026-10-28-refactor-pre-phase6.md` に保存して最初にコミット
- 下の項目ごとにコミット（各コミットでテストを通す）。最後に1つの PR

## backend
**B1. 参加のルールを1つのモジュールに**（`src/progress.rs` を新しく）
- `routes/topics.rs` から `Participation`・`progress`・`voted_counts`・`ensure_in_municipality` を移す。`routes/participation.rs` の `next_statement`・`vote` はこれを使う（写しを消す）
- 分析を見られるかの判定を `analysis_unlocked(&Progress) -> bool` 1つにし、`routes/analysis.rs` の `topic_analysis`・`municipality_consensus` はこれを使う（いまのルールのまま）
- `RESIDENCY_TYPES` を1か所に
- `topics::detail` は、そのトピックの分だけを数える（自治体の全トピックを数えない）。`municipality_consensus` の進み具合をトピックごとに問い合わせる処理（N+1）を1回にまとめる

**B2. 状態を型に**（`src/status.rs`）
- `TopicStatus`（draft / published / archived）・`StatementStatus`（pending / approved / rejected / flagged）の enum と `as_str()`。SQL の直書きを `bind` に置き換える（`moderation::Verdict::status()` もこれを返す）

**B3. CLI を分ける**
- `clap`（derive。MIT/Apache-2.0。引数の読み方の写しをなくし、`--help` を自動で出すため）を足す
- `main.rs` は起動とサブコマンドの振り分けだけ。各コマンドは `src/cli/{seed,analyze,moderation,topics,tag,collect,budget,import,eval}.rs`
- コマンド名・引数・出力は今のまま（`docs/cli.md` と同じ）。`--help` が出るようになったことだけ `docs/cli.md` に書く

**B4. 次の意見の応答に `analysis_unlocked` を足す**
- `GET next-statement` の `progress` に `analysis_unlocked: bool`（B1 の判定）。frontend はこれを見て分析を読み直す（`voted >= total` の写しをやめる）。ADR 0005 の数を変えるときに backend だけを直せばよくなる

## frontend
**F1. API の呼び方をそろえる**
- `lib/api-paths.ts`: `municipalityApi(slug)`・`topicApi(slug, id)` など、URL の組み立てを1か所に
- `API_INTERNAL_URL` の既定値を1か所に（`next.config.ts`・`api-server.ts`・`proxy.ts`）
- `apiGet` も `{error: {code, message}}` を読んで `ApiError` を投げる。サーバー側で Cookie を渡す処理を1つの関数にし、トピックの画面でも渡す
- `PostedStatement` を `types.ts` に（`statement-form.tsx` の独自の型をやめる）

**F2. 取得の処理を1つに**
- `lib/use-api.ts`（読み込み・エラー・やり直し・画面を離れたあとの応答を捨てる）を、`voting-panel.tsx`・`analysis-panel.tsx` で使う
- エラーの文言の取り出し `errorMessage(e, 既定の文)` を1つに（3か所）
- `topic-interaction.tsx` は `analysis_unlocked` を見る（B4）

**F3. よく使う見た目を部品に**（`components/ui/`）
- `Chip`（ボタンとリンク。押した状態）、`SegmentedSwitch`（`finance-header.tsx` から）、`Badge`（黄色の印。予算・新規/拡充・投票中）、`EmptyState`、`ScrollRow`（横スクロールの行。`data-swipe-ignore` 込み）、ボタンのクラス（主・副）
- 写しを置き換える（topic-feed・year-chips・budget-projects・finance-trend・topic-finances・group-table・voting-panel・analysis-panel・relation-sheet・statement-form など）。見た目が少しずつ違うもの（チップの大きさ）は、いまの見た目を引数で残す

**F4. UI でないものを lib に**
- `fieldGroups` → `lib/budget-fields.ts`、`tabsFor`・`transitionTo` → `lib/tabs.ts`（`TabNavFallback` も `tabsFor` を使う）、`MAX_STATEMENT_CHARS`・`countChars` → `lib/statement.ts`、新規/拡充の文言を1か所に
- `types.ts` を backend のファイルの順に並べ直し、名前の無い型に名前を付ける。部品と同じ名前の型 `TopicFinances` を `TopicFinanceData` に

## ドキュメント
- CLAUDE.md（リポジトリの構成・backend のモジュールの説明・API の `analysis_unlocked`）、`docs/cli.md`（`--help`）
- 計画の md の末尾に「実装時の変更点」

## 確認
- 各コミットで `cargo test`（`polis_parity` を含む）・clippy・fmt、frontend の lint・typecheck・build
- CLI: 整理の前後で、`budget list`・`topics drafts`・`collect list`・`moderation list`・`budget show otawara 2026` の出力が同じこと（前の出力を取っておいて比べる）
- 開発サーバー: トップ・トピック（投票・意見の投稿・分析）・まちのお金を、前と同じに返すこと。API の応答を整理の前後で取っておいて比べる（`analysis_unlocked` の追加だけが違う）
- 画面の見た目はユーザーにも見てもらう

## 実装時の変更点
- **B2: SQL の状態の文字列は残した**: 27か所の直書きをすべて bind にすると、プレースホルダーの番号を多くのクエリで振り直すことになり、間違いのもとになる。Rust のコードで扱う状態だけを型にし、型の値が DB の CHECK と同じことを `tests/schema.rs` で確かめる。型の名前は、トピックと予算で同じ値なので `TopicStatus` ではなく `Publication`
- **B3: seed と analyze は `src/cli/mod.rs` に置いた**（数行なので別のファイルにしない）。引数を間違えたときの文は clap のもの（英語）になる
- **B4 は B1 と別のコミット**（応答に項目が増えるのはここだけ）
- **F1: `next.config.ts` は `lib/api-base.ts` の関数を使う**（`.env` を読んだあとに値を読むように、定数ではなく関数）
- **F2: 進捗の知らせ（onProgress）は、データが変わったときの effect で出す**（取得の処理を `useApi` にまとめたため）
- **F3: トピックの一覧のタグの行も、ほかの行と同じくスクロールバーを隠す**（`ScrollRow` にそろえたため。見た目の違いはこれだけ）
- **確認**: 整理の前後で、API の応答（21か所）と CLI の出力（6つ）が同じことを確かめた（`analysis_unlocked` の追加を除く）

