# トピックマップのあとのコードの整理

## Context
タイムラインの流れ・類似判定・トピックマップと市民のまとめ（PR #26〜#30）で、機能を足すことを優先してきた。
提言からまとめへの作り直しで残った重複と残りかすを片付け、大きくなったファイルを分ける（2026-10-31 ユーザーと決定。C まで含める）。
**動きは変えない**（API・画面・CLI の入出力は同じ）。

## やること

### A. 決まりを1か所に
1. 合意点の判定（全グループで (A+1)/(S+2) ≥ 0.8）を `process/analysis/disclosure.rs` の関数1つに（今はピン留めの `interface/api/analysis.rs` と、まとめの `process/digest.rs` に別々にある）
2. 関わり方の人数の数え方と二次秘匿を `disclosure.rs` の関数1つに（今はグループの構成の `analysis.rs` と、まとめの `digest.rs` に別々にある）
3. 残りかす: コメントの「提言」（`disclosure.rs`・`app/layout.tsx`）、空のディレクトリ `frontend/src/components/recommendations/`

### B. 読みやすく
4. `components/digest/map.tsx`（623行）を役割ごとに分ける（地図の画面・拡大縮小と移動・吹き出しとカード・タグの一覧と縮む地図の高さ）
5. 日付の書式（`lib/digest.ts`・`proposals/panel.tsx`・`topics/sources.tsx`）を `lib/dates.ts` に1つに
6. 合意点の部品（`ConsensusToggle`・`ConsensusPins`）を、API（`analysis.rs` の consensus）にそろえて `components/analysis/` へ

### C. 大きいファイルを分ける（今回より前から）
7. `interface/api/operator.rs`（849行）を `operator/`（`mod.rs`: 運営者の確かめ・共通の形・自治体とタグ、`topics.rs`、`publishing.rs`、`statements.rs`）に
8. `interface/cli/records.rs`（804行）を動詞ごとに `records/`（`mod.rs`: 対象と引数、`list.rs`・`show.rs`・`edit.rs`（edit / add / remove）・`publish.rs`）に
9. `components/operator/draft-card.tsx`（556行）を、カード・意見の欄・タグの欄に分ける

### 確かめ
- ステップごとに `cargo test`・clippy・fmt、frontend の lint・型チェック・`npm run build`
- CLAUDE.md のリポジトリ構成の説明を直す
