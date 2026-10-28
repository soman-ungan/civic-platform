# backend/src を用途ごとに階層化する

## Context
`backend/src` 直下には、共通の部品（config・db・util）・外とのやりとり（routes・cli・error）・中身の処理（budget・minutes・moderation…）が、20個ほど同じ高さで並んでいる。どれが何のためのものか、ファイルの場所から分からない。
ユーザーの依頼（2026-10-28）: 用途で階層に分ける。
- システム内で共通に使うもの
- インターフェース類
- 内部ロジック類（CLI を段階で分けたのにならう）

今回は**置き場所を変えるだけ**にし、処理の中身は変えない（例外は下の「中身を少し変えるもの」の2つ）。API の応答と CLI の出力は前と同じにする。

## 新しい構成
```
src/
├── main.rs            起動（引数を読み、CLI か API サーバーへ）
├── lib.rs             下の6つの宣言だけ
├── shared/            システム内で共通に使うもの
│   ├── config.rs  db.rs  status.rs  util.rs
│   └── llm/           Claude API の呼び出し（client・pricing）。取り込みと判定の両方が使う
├── interface/         外とのやりとり
│   ├── api/           HTTP API（今の routes/。AppState・app() は lib.rs から、error.rs・participant.rs もここへ）
│   └── cli/           運営のコマンド（今の src/cli/ をライブラリに移す。main.rs から使う）
├── ingest/            段階1: 資料を DB に入れる（fetch / extract / import）
│   ├── collect/       取得（fetch・robots・store・settlement_card・link_list）
│   ├── budget/        予算の概要の読み取り・検査・下書きの保存（pdftext）
│   ├── minutes/       会議録の読み取り・検査・下書きの保存
│   └── extraction.rs  AI の出力の出どころ（API / 共同作業）
├── records/           段階2: DB を見る・直す（list / show / edit / add / remove / publish / archive）
│   ├── topics.rs      今の drafts.rs（下書きの確認・手直し・公開）
│   ├── budget.rs      予算の一覧・読み出し・公開（今の budget/mod.rs の list・load・set_status）
│   └── settlement.rs  minutes.rs  statements.rs  tags.rs  sources.rs   ← CLI に直書きした SQL を移す
├── process/           段階3: DB の中身を判定・計算する（moderate / classify / analyze）
│   ├── moderation.rs  tagging.rs（classify）
│   ├── analysis/      Polis の数理（PCA・クラスタリング・代表意見・合意点）
│   └── progress.rs    参加のルール（分析を見られるか・参加できるか）
└── maintenance/       準備と保守（CLI の「準備と保守」と同じ）
    ├── seed.rs
    └── eval.rs
```

- 名前: 共通の層を `core` にしないのは、Rust の標準の `core` と名前がぶつかるため（`shared` にする）
- CLI をライブラリに移す理由: 2つのインターフェース（API と CLI）を同じ場所に並べるため。`main.rs` は `backend::interface::cli` を呼ぶだけになる
- 依存の向き: `interface` → `ingest` / `records` / `process` / `maintenance` → `shared`。内部ロジックは `interface` を使わない（下の progress の直しは、このため）。`records` は `ingest` の型（`budget::Checked`）を読み出しに使う
- 予算の表示（`render`）は、取り込みの検査の結果を見せるものなので `ingest/budget` に残す（`extract`・`import`・`show budget` が使う）
- 互換のための再エクスポート（古いパスで呼べるようにするもの）は作らない。テストの `use` を新しいパスに直す

## 中身を少し変えるもの
1. **`progress.rs` が API のエラー型（`ApiError`）を返さないようにする**: 内部ロジックがインターフェースに依存しないように。`sqlx::Error` を返し、`ensure_topic` は「見つかったか」（bool）を返す。404 にするのは API 側（呼び出しの3か所）
2. **CLI の `records.rs` に直書きした SQL を `records/` に移す**（決算カード・会議録・意見・タグ・収集先の一覧と詳細）。CLI は表示だけにする（前の整理の決まり「中身はライブラリのモジュールに任せる」に戻す。フェーズ6の管理画面でも同じ関数を使える）

## 今回はしないこと（気づいたことの記録）
- `interface/api/analysis.rs` の二次秘匿（`suppress`）や、`finances.rs` の集計は、ルールなので `process` に移す候補。量が多いので、別の回にする

## 進め方（コミットの単位。各コミットで cargo test・clippy・fmt が通る）
0. 計画を `docs/plans/2026-10-28-backend-layers.md` に保存してコミット（ブランチ `refactor/backend-layers` を `refactor/cli-verbs` の上に作る）
1. `shared/`（`git mv` と `use` の直し）
2. `interface/`（routes → api、error・participant・AppState、cli をライブラリへ）
3. `ingest/`
4. `process/`（progress の直しを含む）
5. `records/`（drafts → topics、budget の分割、CLI の SQL を移す）
6. `maintenance/`
7. ドキュメント: CLAUDE.md（リポジトリの構成・パス）、**NOTICE（Polis 由来のファイルのパス。AGPL の出典表示）**、`docs/polis-math.md`、`docs/design/collection.md`・`tagging-criteria.md`、ADR 0003。計画の md の末尾に「実装時の変更点」

## 確認
- 各コミットで `cargo test`（`polis_parity` を含む）・clippy・fmt
- 前と後で比べる（API キーは空にして）:
  - CLI: 前の確認と同じ一覧・詳細の出力（`list budget`・`show budget`・`list topic --drafts`・`show topic`・`list source`・`list statement` など）が同じこと
  - API: 使い捨ての DB（`seed --dev`）でサーバーを動かし、主な応答（自治体・トピック一覧と詳細・次の意見・分析・まちのお金・合意点）を整理の前後で取っておいて比べる
- frontend は触らない

## 実装時の変更点
