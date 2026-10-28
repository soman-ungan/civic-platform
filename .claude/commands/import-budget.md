---
description: 当初予算の概要の PDF を、API を使わずに共同作業で予算の下書きにする
argument-hint: <年度（西暦）> [PDF のパス（既定は fetch budget で取った置き場の PDF）] [原本の URL] [自治体の slug（既定 otawara）]
---
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

当初予算の概要を、共同作業で書き写して予算の下書きにする（docs/plans/2026-10-28-manual-extraction.md、phase5c）。
API は呼ばない。出力は API の時と同じ検査（`budget::check`: 合計の検算、事業の名前と金額がそのページの本文にあるか）を通る。

引数: $ARGUMENTS

## 手順
0. PDF が手元に無ければ `cd backend && cargo run -- fetch budget -m <slug> -y <年度>` で取る（置き場 `data/private/fetched/<slug>/budget/<年度>.pdf`。AI は呼ばない）。手で取った PDF なら、以降のコマンドに `--file <PDF>`（と `--url <URL>`）を付ける
1. `cargo run -- extract budget -m <slug> -y <年度> [--file <PDF>] --manual`
   - **`--manual` を必ず付ける**（付けないと API を呼び、費用がかかる）
   - 書き出された `<名前>.prompt.md`（指示・出力の形・分野の語彙）を読む
2. PDF を読む（長いので20ページずつ）。`# 指示` に厳密に従って、`# 出力の形` の JSON を `<名前>.extraction.json`（PDF の隣）に書く
   - 金額は千円の整数と、資料の表記そのまま（amount_text）。ページは PDF のページ番号
   - 事業の説明の文章は写さない（市の著作物のため。名前・金額・ページ・新規/拡充だけ）
   - 新規・拡充の印は、記号のことも文字の色のこともある。迷ったら運営者にページを見てもらう
3. `cargo run -- import budget -m <slug> -y <年度> [--file <PDF>] --dry-run`（import は API を呼ばない）
   - 合計が合わない区分・捨てられた事業があれば、原本を見直して JSON を直し、もう一度
4. 運営者に結果（款別・性質別・歳入・主な事業の件数・捨てたもの）を見せて確かめてもらう
5. 運営者の了承を得てから `--dry-run` を外して下書きを保存する（置き場の PDF なら、ここで消える）
6. 公開は運営者が `show budget -m <slug> -y <年度>` で確かめてから `publish budget -m <slug> -y <年度>`（ここでは公開しない）
