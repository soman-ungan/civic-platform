---
description: 会議録1日分を、API を使わずに共同作業でトピックの下書きにする（本番に入れる）
argument-hint: <会議録のテキストのパス> <会議名> <開催日 YYYY-MM-DD> [自治体の slug（既定 otawara）] [会議録の URL]
---
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

会議録から、トピックの下書きを共同作業で作り、本番に入れる（ADR 0004・ADR 0009、docs/plans/2026-10-28-manual-extraction.md）。
API は呼ばない。データ化は手元で行い、本番の `import` が検査して下書きにする（本番の DB に書くのはそこだけ）。

引数: $ARGUMENTS

送り先は `DEPLOY_SSH`・`DEPLOY_DIR`（docs/operations/deploy.md の「資料を本番に入れる」）。`deploy/` のスクリプトはリポジトリのルートで動かす。

## 手順
1. 原本（UTF-8 のテキスト）は `data/private/minutes/` に置く（git の管理外。リポジトリに入れない）
2. 手元で `cd backend && cargo run -- extract minutes -m <slug> --file <パス> --title <会議名> --held-on <開催日> --dry-run`
   - AI を呼ばずに、発言者・名簿に無い首長と議員・議事日程を出す（DB には書かない）
   - 名簿に無い首長・議員がいたら止めて、運営者に伝える（`data/seeds/030_officials_*.sql` への登録と、本番へのデプロイが要る）
   - 形式的な議事だけの日なら、取り込むかを運営者に聞く（取り込むなら以降の upload.sh に `--force`）
3. 本番で指示文を作る: `deploy/upload.sh --prompt minutes <パス> -m <slug> --title <会議名> --held-on <開催日> --url <URL>`
   - 指示文の `<existing_topics>` が**本番の**トピックになり、同じ論点の統合（`same_as`）が本番に対して効く
   - 原本の隣に `<名前>.prompt.md` と `<名前>.meta.json` が届く。会議名・開催日・URL は meta に残り、以降は打たなくてよい
   - `<名前>.prompt.md` を全部読む（長いので分けて読む）
4. `# 指示` に厳密に従って、`# 出力の形` の JSON を作り、`<名前>.extraction.json`（原本の隣）に書く
   - 入力の `<existing_topics>` と読み比べ、同じ論点（何を決める・変えるかが同じ。立場や言い回しは問わない）のものは `same_as` にその id を書き、statements には既存のトピックに無い主張だけを書く。印を付けたものは、運営者に理由と一緒に見せる
   - 最初の意見は、そのトピックについて議会で出た、立場や中身の違う主張をすべて書く（数は決めない。同じ主張の言い換えは1件にまとめる）。15件を超えたら、言い換えが無いか運営者に伝える
   - 意見が3件未満でもトピックは残る。立場が偏っていれば、運営者に伝える（反対の立場の意見を作って足さない）
   - 引用（quote）は、発言の本文からそのまま写す。要約・言い換えをしない
   - 最初の意見は、議会で実際に出た主張だけ。新しい選択肢を作らない。意見に氏名を入れない
   - 職員の氏名は入力に無い。推測して足さない
5. 本番で検査する: `deploy/upload.sh minutes <パス> -m <slug>`（本番の `import --dry-run`。保存しない）
   - 「捨てたもの」があれば、原因（引用が原文に無い・氏名が入っている・根拠が無い）を直して、もう一度
6. 運営者に、トピック案（タイトル・説明・引用・最初の意見・統合の印）を見せて確かめてもらう。直す点があれば JSON を直し、5 からやり直す
7. 運営者の了承を得てから `deploy/upload.sh --apply minutes <パス> -m <slug>` で下書きにする
   - 公開中のトピックに足した意見は確認待ち（flagged）になる。採否は運営者が管理画面（`/operator`）の「確認待ち」で行う（ここでは処理しない）
   - 最初の意見は本番の AI の審査を通る（費用は小さい）
8. タグ: 本番では自動のタグ付けが動く（1分ほど）。`deploy/prod.sh show topic --id <id>` で確かめ、足りなければ分野の語彙（`deploy/prod.sh list tag -m <slug>`、基準は `docs/design/tagging-criteria.md`）から案を出し、運営者の了承を得て `deploy/prod.sh add tag --topic <topic_id> --id <tag_id>`
9. 公開は運営者が管理画面（`/operator`）で確かめて行う（公開待ちに入れるか、すぐ公開）。ここでは公開しない

会議録の原本と、作った JSON・prompt・meta は `data/private/` に置いたままにする（手でコピーした原本は import でも消えない）。

**手元の DB で試すとき**（開発用。手元の DB は本番と別物）: 3 を `cargo run -- extract minutes … --manual`、5・7 を `cargo run -- import minutes -m <slug> --file <パス> [--dry-run]` に置き換える。手元の `<existing_topics>` は本番と違うので、本番に入れる JSON は必ず 3 の本番の指示文で作る。
