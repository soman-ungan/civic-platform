# docs の構造化

## Context
`docs/` に47ファイルが並び、読む人（運営者・開発者・Claude）ごとに何を読めばよいかが分かりにくい。
- `grand-design.md` が古い。CLAUDE.md は「設計判断で迷ったら必ず読み直す」「テーブルの定義は §3」としているが、§3 には `documents`・`topic_sources`・`passkeys`・予算と決算・`topic_analyses`・`llm_usage`・`public_officials` などが無い（マイグレーションは17本）
- CLAUDE.md（198行）に API の一覧とコマンドの細部があり、コードや `docs/cli.md` と二重になっている。毎回読み込まれる
- `plans/` は日付順に31ファイル（約2,100行）。今も効いている内容と過去の経緯が見分けにくい
- `cli.md` と `data-flow.md` は、どちらも資料ごとの流れを持っていて一部が重なる

ユーザーの依頼（2026-10-29）: 要約するか、ディレクトリで構造化するかを検討する。

## 決めたこと（2026-10-29 ユーザーと決定）
- **要約ではなく構造化と、古い文書の書き直し**（計画・ADR は記録なので中身を直さない）
- `grand-design.md` は今の姿に書き直す（新しい `overview.md`。テーブルの定義は `design/schema.md` に分ける）
- CLAUDE.md の API の一覧とコマンドの細部は docs に移し、原則と「どこを読むか」の案内を中心にする
- `plans/` は中身をそのままにし、索引（`plans/README.md`）を作る

## 制約: マイグレーションから参照されているパスは動かさない
main にマージ済みのマイグレーションは書き換えられない（sqlx のチェックサム）。コメントで次のパスを参照しているので、動かさない:
`docs/adr/0002-*`・`docs/design/collection.md`・`docs/design/tagging-criteria.md`・`docs/polis-math.md`・`docs/plans/*`（複数）・`docs/grand-design.md`（「v2 §3」）

そのため `grand-design.md` は当初の構想（v2）として残し、冒頭に「今の姿は overview.md と design/schema.md」と注記するだけにする。

## 新しい構成
```
docs/
├── README.md            新規: 入口。読む人ごとに何がどこにあるか
├── overview.md          新規: 今の全体像（思想・構成・原則・フェーズの状況）
├── grand-design.md      当初の構想 v2（冒頭に注記だけ）
├── polis-math.md        そのまま
├── operations/          運営者向け: cli.md・data-flow.md（← docs/ 直下から移す）。重なりを整理する
├── design/              開発者向け
│   ├── schema.md        新規: 今のテーブルと関係・状態の値（マイグレーションから）
│   ├── api.md           新規: CLAUDE.md の API の節を移す
│   ├── collection.md    そのまま
│   └── tagging-criteria.md  そのまま
├── adr/                 そのまま
├── plans/               中身はそのまま ＋ README.md（フェーズごとの索引と、今も効く内容が今どこにあるか）
└── outreach/            そのまま
```

## 手順（それぞれコミット）
1. `cli.md`・`data-flow.md` を `operations/` に移し、参照を直す（CLAUDE.md・README.md・コードのコメント・ADR 0004・2つの間のリンク）。`plans/` の中の古いパスは直さない
2. `design/schema.md`・`design/api.md` を作る
3. `overview.md` を作り、`grand-design.md` に注記を足す
4. CLAUDE.md を軽くする（API → `design/api.md`、コマンドの細部 → `operations/cli.md`。「迷ったら読む」先を overview・ADR・schema に）
5. `docs/README.md`・`plans/README.md`

## 確認
- リンク切れの検査: md の中の相対リンクと、コード・CLAUDE.md・README の中の `docs/…` の参照がすべて存在する
- `cargo test`（マイグレーションに触れていないこと）

## しないこと
- 計画・ADR の中身の書き換え、要約
- マイグレーションから参照されているファイルの移動

## 実装時の変更点
- `cli.md` と `data-flow.md` の重なりは、書き直さずに役割（手順と、資料がどこに入るか）を冒頭で示し合う形にした。`data-flow.md` の「1年の作業の目安」を管理画面に合わせた
- CLAUDE.md から外したコマンドの細部のうち、`docs/operations/cli.md` に無かった AI のモデルの環境変数（`MODERATION_MODEL` など）を cli.md に足した。開発者向けのもの（`.env` の準備、サンプルの投票の作り方、コマンドを足す場所）は CLAUDE.md に残した
- CLAUDE.md の開発フェーズは、終わったものを1行にした。6c の「市民からの提案」は、実際の分け方に合わせて 6d に書いた
- ルートの README.md の「設計」の行を docs の入口に変えた
- 確認: md の相対リンクと見出し、コード・CLAUDE.md・README の `docs/…` の参照を、スクリプトで確かめた（`plans/` の本文の古いパスは記録なので除く）。このブランチはマイグレーションに触れていない
