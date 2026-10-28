---
description: 会議録1日分を、API を使わずに共同作業でトピックの下書きにする
argument-hint: <会議録のテキストのパス> <会議名> <開催日 YYYY-MM-DD> [自治体の slug（既定 otawara）]
---
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

会議録から、トピックの下書きを共同作業で作る（docs/plans/2026-10-28-manual-extraction.md、ADR 0004）。
API は呼ばない。出力は API の時と同じ検査（`minutes::extract::check`）を通る。

引数: $ARGUMENTS

## 手順
1. `cd backend && cargo run -- extract minutes -m <slug> --file <パス> --title <会議名> --held-on <開催日> --dry-run`
   - AI を呼ばずに、発言者・名簿に無い首長と議員・議事日程を出す
   - 名簿に無い首長・議員がいたら止めて、運営者に伝える（`data/seeds/030_officials_*.sql` への登録が要る）
   - 形式的な議事だけの日なら、取り込むかを運営者に聞く（取り込むなら import に `--force`）
2. `cargo run -- extract minutes -m <slug> --file <パス> --title <会議名> --held-on <開催日> --manual`
   - **`--manual` を必ず付ける**（付けないと API を呼び、費用がかかる）
   - 書き出された `<名前>.prompt.md` を全部読む（長いので分けて読む）。会議名・開催日は原本の隣の `<名前>.meta.json` に残り、以降は打たなくてよい
3. `# 指示` に厳密に従って、`# 出力の形` の JSON を作り、`<名前>.extraction.json`（原本の隣）に書く
   - 引用（quote）は、発言の本文からそのまま写す。要約・言い換えをしない
   - 最初の意見は、議会で実際に出た主張だけ。新しい選択肢を作らない。意見に氏名を入れない
   - 職員の氏名は入力に無い。推測して足さない
4. `cargo run -- import minutes -m <slug> --file <パス> --dry-run` で検査に通す（import は API を呼ばない）
   - 「捨てたもの」があれば、原因（引用が原文に無い・氏名が入っている・根拠が無い）を直して、もう一度
5. 運営者に、トピック案（タイトル・説明・引用・最初の意見）を見せて確かめてもらう。直す点があれば JSON を直す
6. 運営者の了承を得てから `cargo run -- import minutes -m <slug> --file <パス>` で下書きを保存する
   - 最初の意見はモデレーションに通る。API キーがあれば AI が判定する（費用は小さい）。無ければ承認される
7. タグ: API キーが無ければ自動のタグ付けは動かない。分野の語彙（`cargo run -- list tag -m <slug>`、基準は `docs/design/tagging-criteria.md`）から案を出し、運営者の了承を得て `cargo run -- add tag --topic <topic_id> --id <tag_id>`
8. 公開は運営者が `show topic --id <id>` で確かめてから `publish topic --id <id>`（ここでは公開しない）

会議録の原本と、作った JSON・prompt・meta は `data/private/`（git の管理外）に置いたままにする（手でコピーした原本は import でも消えない）。
