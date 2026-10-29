# 評価用データ（モデレーション・自動タグ付け）

AI のモデルと設定を決めるための評価用データ（`docs/plans/2026-10-27-phase4-ai.md` Step 4）。

- `moderation.jsonl` — 投稿の本文と、正しい判定（`approved` / `rejected` / `flagged` / `duplicate`）。`note` は何を確かめるケースか
  - `rejection`（任意）: 非公開の理由の種類（`harm` / `person` / `not_votable`）。書けば理由も合っているかを見る
  - `topic`（任意）: 投稿先のトピックの `title`・`description`。無ければ、投票できる意見かを文の形だけで見る
  - `statements`（任意）: トピックの既存の意見の本文（似た意見の判定。`duplicate` のケースには要る）
  - 公人の氏名（名簿との一致）はプログラムで確かめるので、ここでは評価しない（AI の「人物」の観点だけを見る）
- `tagging.jsonl` — トピックのタイトル・説明と、正しい分野タグ・地区タグ（名前で書く）

## 注意

- **人名はすべて架空**（山田一郎・鈴木花子・田中次郎など、例示によく使われる名前）。実在の首長・議員・住民とは関係ない。電話番号や番地も実在しない形にしている
- 実在の人名を入れない。公開リポジトリに載るため
- 判定の基準は `backend/src/process/moderation.rs` の指示文と ADR 0007 に合わせる（主張や政治的な立場では判断しない、公人の公務・政策への批判は公開、人格攻撃・私生活・人物の評価や進退は非公開、投票できない文・無関係な文は非公開、同じ意見は重複）
- 地名・施設名（黒羽・湯津上・雲巌寺・那珂川など）は大田原市の実在の地名だが、トピックの内容は架空

## 実行

実際の API を呼ぶので費用がかかる（目安は ADR 0003）。`.env` に `ANTHROPIC_API_KEY` を設定し、`data/seeds/` を投入した DB で実行する。

```sh
cd backend
cargo run -- eval moderation [--model claude-haiku-5-5] [--limit 10]
cargo run -- eval tagging [--model claude-sonnet-5-5]
```

使用量は `llm_usage` に `purpose = 'eval'` で記録され、1日の上限（`LLM_DAILY_BUDGET_USD`）にも数えられる。
