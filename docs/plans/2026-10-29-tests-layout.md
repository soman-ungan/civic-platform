# backend/tests を1つのテストバイナリにまとめ、src の階層にそろえる

## Context
`backend/tests/` の26ファイル（185テスト）は、ファイルごとに別のテストバイナリになっている。
- backend 全体を26回リンクする。1つ約190MB（デバッグ情報つき）で、ビルドのたびに約5GB。`target/` は 37GB。`src/` を触ったあとのテストのビルドが約21秒
- テストごとに使う関数が違うので、`common/mod.rs` に `#[allow(dead_code)]` が14個ある
- 名前がそろっていない（`api_topics.rs` と `operator.rs`・`auth.rs`）。`src/` の階層（interface・ingest・process・shared・maintenance）と対応していない
- 冒頭の説明（`//!`）が無いファイルが8つ

ユーザーの依頼（2026-10-29）: `backend/tests/` の整理が必要か検討し、この形で進める。

## 新しい構成
```
tests/
├── it/                       1つのバイナリ（cargo test --test it）
│   ├── main.rs               mod を並べるだけ
│   ├── common.rs             ← common/mod.rs（allow(dead_code) を外す）
│   ├── api/                  health・topics・participation・analysis・finances・auth・operator・participant・drafts
│   ├── ingest/               collect_fetch・collect_settlement_card・settlement_card・budget・minutes
│   ├── process/              analysis_store・moderation・review・tagging・participation_rules・topic_deadlines
│   ├── shared/               llm・schema
│   └── maintenance/          seeds・eval
├── polis_parity.rs           別のまま（docs と CLAUDE.md が `cargo test --test polis_parity` を案内している）
├── cli.rs                    別のまま（実際のバイナリを動かす。common は #[path] で it/common.rs を使う）
└── fixtures/
```
- 置き場所は、主に確かめている層で決める。API を通して参加のルール（`process/progress.rs`）を確かめるもの（participation_rules・topic_deadlines）は process
- ファイル名は `api_` を外すだけにし、ほかは今の名前のまま（履歴を追いやすくする。`git mv`）
- `include_str!` の相対パス（budget.rs）は `CARGO_MANIFEST_DIR` からにする（ファイルの深さに左右されないように）
- 同じプロセスで並んで動くようになるが、テストは環境変数・固定のポート・共有のファイルを使っていない（ポートは `:0`、一時ディレクトリはテストごとの名前）

## 変えないこと
- テストの関数と、確かめていること

## ドキュメント
- 1つのテストを動かすときは `cargo test --test it <名前>`（例: `cargo test --test it seeds`）。`--test seeds` を案内している CLAUDE.md を直す。CLAUDE.md にテストの構成を1行書く

## 確認
- `cargo test` で、テストの数がまとめる前と同じ（185）で、すべて通る
- clippy の警告なし
- まとめる前と後で、`src/` を触ったあとのテストのビルドの時間と、テストのバイナリの大きさを比べる
