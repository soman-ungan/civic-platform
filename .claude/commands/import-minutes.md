---
description: 会議録1日分を、API を使わずに共同作業でトピックの下書きにする
argument-hint: <会議録のテキストのパス> <会議名> <開催日 YYYY-MM-DD> [自治体の slug（既定 otawara）]
---
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

会議録から、トピックの下書きを共同作業で作る（docs/plans/2026-10-28-manual-extraction.md、ADR 0004）。
API は呼ばない。出力は API の時と同じ検査（`minutes::extract::check`）を通る。

引数: $ARGUMENTS

## 手順
1. `cd backend && cargo run -- import minutes --municipality <slug> --file <パス> --title <会議名> --held-on <開催日> --dry-run`
   - 名簿に無い首長・議員がいたら止めて、運営者に伝える（`data/seeds/030_officials_*.sql` への登録が要る）
   - 形式的な議事だけの日なら、取り込むかを運営者に聞く（取り込むなら以降のコマンドに `--force`）
2. `cargo run -- extraction prompt minutes --municipality <slug> --file <パス> --title <会議名> --held-on <開催日>`
   - 書き出された `<名前>.prompt.md` を全部読む（長いので分けて読む）
3. `# 指示` に厳密に従って、`# 出力の形` の JSON を作り、`<名前>.extraction.json` に書く
   - 引用（quote）は、発言の本文からそのまま写す。要約・言い換えをしない
   - 最初の意見は、議会で実際に出た主張だけ。新しい選択肢を作らない。意見に氏名を入れない
   - 職員の氏名は入力に無い。推測して足さない
4. `cargo run -- import minutes ... --extraction <JSON> --dry-run` で検査に通す
   - 「捨てたもの」があれば、原因（引用が原文に無い・氏名が入っている・根拠が無い）を直して、もう一度
5. 運営者に、トピック案（タイトル・説明・引用・最初の意見）を見せて確かめてもらう。直す点があれば JSON を直す
6. 運営者の了承を得てから `cargo run -- import minutes ... --extraction <JSON>` で下書きを保存する
7. タグ: API キーが無ければ自動のタグ付けは動かない。分野の語彙（`data/seeds/020_field_tags.sql`、基準は `docs/design/tagging-criteria.md`）から案を出し、運営者の了承を得て `cargo run -- topics tag add <topic_id> <tag_id>`
8. 公開は運営者が `topics show <id>` で確かめてから `topics publish <id>`（ここでは公開しない）

会議録の原本と、作った JSON・prompt は `data/private/`（git の管理外）に置いたままにする。
