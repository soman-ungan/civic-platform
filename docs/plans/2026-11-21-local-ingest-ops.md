# 資料は手元でデータ化し、本番は検査と保存だけにする（運用に合わせた整理）

## Context
2026-11-21、ユーザーの相談。当初の構想（`docs/grand-design.md`・フェーズ5）では、本番のサーバーが資料を集めて AI で読み、
取り込みを定期的に回すつもりだった。公開（2026-11-10）からの実際の運用は次のとおりで、こちらのほうが管理しやすい:

- 会議録・予算は、手元で Claude Code との共同作業（`extract --manual`）で JSON にする（API の費用なし）
- `deploy/upload.sh` で本番に送り、本番の `import` が検査して下書きにする（本番の DB に書く窓口は1つ）
- 会議録の指示文は本番で作る（`--prompt`。本番の既存のトピックとの統合 `same_as` が効く）
- 決算カードは、本番のサーバーが総務省から取る（AI を使わない決まった処理）

コードはおおむねこの形になっているので、作り直しはしない。当初の構想の名残と、本番の操作の手間だけを直す。

## やること
1. **ADR 0009**: 資料のデータ化は手元（共同作業）で、本番は検査と保存だけ。定期の収集はしない。
   API で読む `extract` は手元だけに残す。ADR 0003（AI の運用）・CLAUDE.md・overview の該当箇所を直す
2. **API で読む `extract` を本番で動かさない**
   - 環境変数 `EXTRACTION_API`（既定 `on`）。`off` なら `extract`（`--manual`・`--dry-run` を除く）は、API を呼ばずにエラーで止まり、`--manual` を案内する
   - `deploy/compose.yaml` で `EXTRACTION_API: "off"` を固定し、`EXTRACTION_MODEL`・`LLM_EXTRACTION_DAILY_BUDGET_USD` を本番の値から外す（`deploy/.env.example` も）
3. **「収集の定期実行」を予定から外す**（CLAUDE.md・overview・deploy.md）。会議録は規約で自動で取れず、予算と決算は年1回のため
4. **本番のコマンドの薄いラッパー `deploy/prod.sh`**: `deploy/prod.sh list topic -m otawara` で、本番の backend のコンテナでコマンドを動かす
   （`DEPLOY_SSH`・`DEPLOY_DIR` は `upload.sh` と同じ）。`upload.sh` と同じく、秘密の値や IP はスクリプトに書かない
5. **AI の利用を見るコマンド `list usage`**: 日ごと（UTC）・用途ごとの呼び出し回数・トークン・推定の費用・失敗の数（`llm_usage`）。
   `--days <n>`（既定 7）。本番のコンテナには sqlite3 が無く、キーを差し替えたあとに確かめる手段が無かったため
6. **取り込みの手順書を本番を本筋に**: `.claude/commands/import-minutes.md`・`import-budget.md` を、
   `upload.sh --prompt` → 共同作業 → `upload.sh`（検査）→ 運営者の確認 → `upload.sh --apply` の流れに書き直す。
   手元の DB に保存する手順は、開発で試すとき用として短く残す
7. docs: `operations/cli.md`（`list usage`・`EXTRACTION_API`）・`operations/deploy.md`（`prod.sh`・資料の入れ方）・`operations/data-flow.md`

## しないこと
- `<動詞> <対象>` の形と、段階ごとの置き場所（ingest・records・process）は変えない。表の「無」（何もしない組み合わせ）もそのまま
- API で読む処理（`Extraction::Api`）は消さない（手元で使える）

## 確かめ
- `cargo test`（`EXTRACTION_API=off` で `extract` が止まること、`list usage` の集計のテストを足す）・clippy・fmt
- `deploy/prod.sh` で本番の `list topic`・`list usage` を読む（読むだけ）
