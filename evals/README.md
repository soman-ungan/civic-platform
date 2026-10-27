# 評価用データ（モデレーション・自動タグ付け）

AI のモデルと設定を決めるための評価用データ（`docs/plans/2026-10-27-phase4-ai.md` Step 4）。

- `moderation.jsonl` — 投稿の本文と、正しい判定（`approved` / `rejected` / `flagged`）。`note` は何を確かめるケースか
- `tagging.jsonl` — トピックのタイトル・説明と、正しい分野タグ・地区タグ（名前で書く）

## 注意

- **人名はすべて架空**（山田一郎・鈴木花子・田中次郎など、例示によく使われる名前）。実在の首長・議員・住民とは関係ない。電話番号や番地も実在しない形にしている
- 実在の人名を入れない。公開リポジトリに載るため
- 判定の基準は `backend/src/moderation.rs` の指示文（主張や政治的な立場では判断しない、公人の公務への批判は公開、人格攻撃・私生活は非公開）に合わせる
- 地名・施設名（黒羽・湯津上・雲巌寺・那珂川など）は大田原市の実在の地名だが、トピックの内容は架空

## 実行

実際の API を呼ぶので費用がかかる（目安は ADR 0003）。`.env` に `ANTHROPIC_API_KEY` を設定し、`data/seeds/` を投入した DB で実行する。

```sh
cd backend
cargo run -- eval moderation [--model claude-haiku-5-5] [--limit 10]
cargo run -- eval tagging [--model claude-sonnet-5-5]
```

使用量は `llm_usage` に `purpose = 'eval'` で記録され、1日の上限（`LLM_DAILY_BUDGET_USD`）にも数えられる。
