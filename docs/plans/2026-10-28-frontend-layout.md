# frontend/src を用途ごとに階層化する

## Context
`frontend/src/components/` の直下に37個、`lib/` の直下に17個のファイルが同じ高さで並び、どの画面のものかがファイルの場所から分からない。
`lib/types.ts` のコメントは、backend を階層化する前のパス（`backend/src/routes/...`）のままになっている。
ユーザーの依頼（2026-10-28。フェーズ6b の前に）: frontend のファイル構成を見直す。
- まとまりの名前は backend の `interface/api/` のモジュールにそろえる（画面と API を突き合わせやすくする）
- `types.ts` は backend の API のまとまりごとに分ける
- ファイル名のうち、ディレクトリ名と重なる部分は外す（名前が重なっても、ディレクトリが違えば問題ない）

今回は**置き場所・ファイル名・import だけを変える**。画面の見た目と動き、部品の名前（`export` する名前）は変えない。

## 新しい構成
```
src/
├── app/                   ルーティング（今のまま）
├── proxy.ts               （今のまま。Next.js が決める置き場所）
├── components/
│   ├── ui/                よく使う見た目: chip・empty-state・scroll-row・segmented-switch・styles・info-tip（← components/）
│   ├── layout/            画面の枠: site-footer・tab-nav・swipe-nav・page-slide
│   ├── topics/            トピックの一覧と詳細: feed・interaction・sources・finances（← topic-*）・consensus-pins
│   ├── participation/     投票・投稿・関わり方: voting-panel・statement-form・relation-sheet
│   ├── analysis/          分析: panel（← analysis-panel）・opinion-map・group-table・group-composition・residency-dot-plot
│   ├── finances/          まちのお金: bars・carousel・donut・header・nature・sections・sources・trend（← finance-*）・budget-comparison・budget-projects・budget-treemap・year-chips
│   └── auth/              パスキー・運営者: account-links・login-sheet・passkey-choices・passkey-prompt・operator-join
└── lib/
    ├── api/               backend とのやりとり: base・client・error・paths・server（← api-*）・use-api
    │   └── types/         API の型（← types.ts）: municipalities・topics・participation・analysis・finances・auth
    ├── finances/          お金の表示の決まり: terms・view（← finance-*）・budget-fields・money・treemap
    ├── auth/passkey.ts
    └── labels.ts・statement.ts・tabs.ts・use-dismiss.ts（画面をまたいで使う小さな決まり）
```

- 型のファイルは backend の `interface/api/` のファイルと同じ名前にし、コメントに今の backend のパスを書く
  - 参加のルール（`Participation`・`Progress`。backend は `process/progress.rs`）と意見の状態（`StatementStatus`。`shared/status.rs`）は、それを返す API の `participation` に置く
  - ログインの状態（`Me`）は `auth`、エラーの本文（`ApiErrorBody`）は使う場所の `lib/api/error.ts` に移す
  - まとめて読み込み直すためのファイル（`index.ts` で全部を export し直すもの）は作らない（backend の整理と同じ。どのまとまりの型かを import に残す）
- import は、同じディレクトリの中なら `./`、ほかは `@/` に統一する
- `next.config.ts` が読む `src/lib/api-base.ts` は `src/lib/api/base.ts` に変わる

## 進め方（コミットの単位。各コミットで lint・typecheck・build が通る）
0. 計画をこの md に保存してコミット（ブランチ `refactor/frontend-layout`）
1. `components/` と `lib/` のファイルを `git mv` で移し、import を直す
2. `types.ts` を分ける
3. ドキュメント: CLAUDE.md（リポジトリの構成）、計画の md の末尾に「実装時の変更点」

## 確認
- lint・typecheck・build
- 前と後で build のルートの一覧が同じこと
- 開発サーバーで主な画面（トップ・トピックの詳細と投票・分析・まちのお金・`/operator/join`）を開き、表示が変わらないこと

## 実装時の変更点
- 移すのと import の書き換えは、一度だけ使うスクリプトで行った（移す前のパスで import の行き先を解決し、移した先が同じディレクトリなら `./`、ほかは `@/` にした）。import の並びは prettier が直した
- `types.ts` を分けるとき、元のファイルで「意見を投稿した結果」の説明が `Me` の上にずれていたので `PostedStatement` に戻した。コメントの backend のパス（`backend/src/status.rs` など）も今のものに直した
- 過去の計画の md（`docs/plans/`）の中の古いパスは、その時点の記録なので直さない

