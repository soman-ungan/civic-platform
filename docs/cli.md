# backend のコマンド一覧（運営者向け）

backend（Rust）は、API サーバーとしてだけでなく、運営のためのコマンドとしても使います。
自治体のデータの投入、会議録・予算の取り込み、AI の判定の確認などは、すべてこのコマンドで行います。
管理画面は、認証（開発フェーズ6）ができるまではありません。

- 開発中は `backend/` で `cargo run -- <コマンド>` として実行します
- ビルドしたものは `./backend <コマンド>`（`cargo build --release` なら `target/release/backend`）でも同じです
- 引数を付けずに実行すると、API サーバーが起動します
- どのコマンドも、起動のたびに DB のマイグレーションを自動で適用します

## 準備

```sh
cp .env.example .env          # リポジトリのルートに1つだけ置く
openssl rand -hex 32          # 出てきた値を .env の SESSION_SECRET に書く（必須）
cd backend
cargo run -- seed             # 都道府県・自治体・分野タグなどのマスタを入れる
cargo run                     # API サーバーを起動（http://127.0.0.1:8080）
```

### 必要なもの

| もの | 使うコマンド | 無いとき |
|---|---|---|
| `SESSION_SECRET`（32バイト以上） | すべて | 起動しない |
| `ANTHROPIC_API_KEY`（Claude API） | AI の判定・タグ付け・会議録と予算の取り込み・評価 | AI を使わない。投稿は自動で承認され、タグ付けはしない。AI が要るコマンドはエラーになる |
| `COLLECTOR_CONTACT`（運営者の連絡先） | `collect run` | 収集しない（取得先の User-Agent に入れるため） |
| poppler-utils（`pdftotext`） | `import budget`、予算の `collect run` | 予算を取り込めない（`apt install poppler-utils`） |

### 費用がかかるコマンド

Claude API を呼ぶコマンドは費用がかかります（料金は Anthropic の API の料金）。目安は次のとおりです。

| コマンド | 目安 |
|---|---|
| `import minutes`（会議録1日分） | 0.3〜0.4 ドル |
| `import budget`（当初予算の概要、40ページ） | 約 0.8 ドル |
| `eval moderation` / `eval tagging` | 件数による |
| サーバーのモデレーション・タグ付け | 投稿・トピックの数による（`llm_usage` で確かめられる） |

1日（UTC）の費用には上限があり、超えると AI を呼びません。

| 環境変数 | 既定 | 対象 |
|---|---|---|
| `LLM_DAILY_BUDGET_USD` | 1.0 | モデレーション・タグ付け・評価 |
| `LLM_EXTRACTION_DAILY_BUDGET_USD` | 5.0 | 会議録・予算の取り込み（別枠） |

使った量と推定の費用は、DB の `llm_usage` に記録されます。

## コマンドの一覧

| コマンド | すること |
|---|---|
| （引数なし） | API サーバーを起動する |
| [`seed`](#seed) | マスタ（自治体・地区・分野タグ・公人の名簿・収集先）を入れる |
| [`analyze`](#analyze) | 意見の分析（Polis の意見マップ・グループ・合意点）をすぐに計算し直す |
| [`moderation`](#moderation) | 確認待ちの意見を見る・承認する・非公開にする |
| [`tag`](#tag) | トピックに分野・地区のタグを AI で付ける |
| [`import minutes`](#import-minutes) | 会議録からトピックの下書きを作る |
| [`topics`](#topics) | トピックの下書きを確かめ、手直しし、公開する |
| [`collect`](#collect) | 行政資料（決算カード・予算の概要）を取得する |
| [`import budget`](#import-budget) | 手元の当初予算の概要の PDF から、予算の下書きを作る |
| [`budget`](#budget) | 予算の下書きを確かめ、公開する |
| [`eval`](#eval) | AI のモデレーション・タグ付けの精度を評価する |

`<slug>` は自治体の URL の名前（例: `otawara`）、`<id>` はトピックや意見の ID です。

---

### seed

```sh
cargo run -- seed          # data/seeds/*.sql をファイル名の順に入れる
cargo run -- seed --dev    # 続けて data/dev-seeds/（【サンプル】の架空のデータ）も入れる
```

- 何度実行しても同じ結果になります（すでにある行は入れ直しません。名簿などは更新します）
- `--dev` は開発用です。架空の参加者120人と投票が入るので、分析の画面をすぐに確かめられます。**本番では使いません**

### analyze

```sh
cargo run -- analyze                  # 投票が変わったトピックだけ
cargo run -- analyze --force          # すべての公開済みのトピック
cargo run -- analyze --topic <id>     # 1つのトピック
```

サーバーは `ANALYSIS_INTERVAL_SECS`（既定10秒）ごとに自動で計算します。このコマンドは、すぐに確かめたいときに使います。

### moderation

```sh
cargo run -- moderation list          # 確認待ち（flagged）と判定待ち（pending）の意見
cargo run -- moderation approve <id>  # 公開する
cargo run -- moderation reject <id>   # 非公開にする
cargo run -- moderation retry         # 判定待ちの意見を、AI でもう一度判定する
```

- AI は、主張や政治的な立場では判断しません。公人の公務への批判は公開し、人格攻撃や私生活に触れるものは非公開にします（`docs/adr/0003-llm-operations.md`）
- 通信の失敗や費用の上限で判定できなかった意見は、自動で承認せず pending のまま残します

### tag

```sh
cargo run -- tag                      # タグの無いトピックすべて
cargo run -- tag --topic <id>         # 1つのトピック
cargo run -- tag --topic <id> --force # AI が付けたタグだけを付け直す（運営者が付けたタグは残す）
```

サーバーは60秒ごとに、タグの無いトピックを自動で処理します。分野の語彙と判断の基準は `data/seeds/020_field_tags.sql` と `docs/design/tagging-criteria.md` にあります。

### import minutes

```sh
cargo run -- import minutes --municipality otawara \
  --file ../data/private/minutes/2026R08_0226.txt \
  --title "令和8年第1回定例会（第2号）" --held-on 2026-02-26 \
  --url "https://..." --dry-run
```

| オプション | 意味 |
|---|---|
| `--municipality <slug>` | 自治体 |
| `--file <path>` | 会議録の本文（UTF-8 のテキスト。git の管理外の `data/private/` に置く） |
| `--title` / `--held-on` | 会議名と開催日（`YYYY-MM-DD`） |
| `--url` | 会議録のページ（画面の出典のリンクに使う。任意） |
| `--dry-run` | AI を呼ばずに、発言者・名簿に無い首長と議員・議事日程だけを出す（**費用なし**） |
| `--again` | 取り込み済みの会議録を作り直す |
| `--force` | 一般質問・代表質問の無い会議録（採決だけの日など）も取り込む |

- **費用がかかります**（1日分でおよそ 0.3〜0.4 ドル）
- 名簿に無い首長・議員がいると止まります。`data/seeds/030_officials_<団体コード>_<slug>.sql` に登録して `seed` してから、やり直してください
- 作られるのは下書きです。[`topics`](#topics) で確かめてから公開します（`docs/adr/0004-assembly-minutes.md`）

### topics

```sh
cargo run -- topics drafts                   # 下書きの一覧
cargo run -- topics show <id>                # 説明・出典の引用・最初の意見・タグ
cargo run -- topics publish <id>             # 公開する
cargo run -- topics archive <id>             # 取り下げる

cargo run -- topics edit <id> --title "..." --description "..."
cargo run -- topics statement add <topic_id> "意見の本文"
cargo run -- topics statement edit <意見の id> "直した本文"
cargo run -- topics statement remove <意見の id>
cargo run -- topics tag add <topic_id> <tag_id>      # 例: field-kosodate、092100-kurobane
cargo run -- topics tag remove <topic_id> <tag_id>
```

- 下書きは、API のどこにも出ません
- 投票がある意見は、直せません（投票の意味が変わってしまうため）
- 意見は140文字以内です

### collect

```sh
cargo run -- collect list                              # 収集先・規約の確認日・最後の取得
cargo run -- collect run                               # 有効な収集先すべて（AI で読むものを除く）
cargo run -- collect run --source 092100-settlement-card
cargo run -- collect run --source 092100-budget-overview --year 2026   # AI で読む（費用がかかる）
cargo run -- collect run --dry-run                     # 取得して確かめるだけで、保存しない
```

| オプション | 意味 |
|---|---|
| `--source <id>` | 1つの収集先だけ |
| `--year <n>` | 1つの年度だけ（西暦） |
| `--dry-run` | 保存しない（AI で読む収集先は、AI は呼ぶ） |
| `--ai` | AI で資料を読む収集先（予算の概要）も動かす。**費用がかかる** |

- 収集先は `collector_sources` の行です。自治体の seed ファイルに書きます（`data/seeds/010_municipality_092100_otawara.sql` を参照）
- 取得は、robots.txt に従い、同じサイトには5秒以上あけ、変わっていない資料は取り直しません
- 利用規約の確認の記録（`terms_url`・`terms_checked_at`）が無い収集先は動きません。確認から1年を過ぎると警告が出ます（`docs/design/collection.md`）
- 総務省の決算カードは、全国の市町村で同じ設定で取得できます（平成27年度以降）

### import budget

```sh
cargo run -- import budget --municipality otawara \
  --file ../data/private/budget/R8yosangaiyou.pdf --year 2026 \
  --url "https://www.city.ohtawara.tochigi.jp/.../R8yosangaiyou.pdf" --dry-run
```

| オプション | 意味 |
|---|---|
| `--municipality <slug>` | 自治体 |
| `--file <pdf>` | 当初予算の概要の PDF |
| `--year <n>` | 年度（西暦） |
| `--url` | 原本の PDF の URL（画面の「説明は原本 p.N」のリンクに使う） |
| `--dry-run` | 読んで確かめた結果を出すだけで、保存しない（AI は呼ぶ） |
| `--again` | 公開済みの年度を作り直す（下書きに戻る） |

- **費用がかかります**（40ページでおよそ 0.8 ドル）
- AI が読んだ款別・性質別・歳入の額は、合計が合わなければ保存しません。主な事業は、名前と金額がそのページの本文にあるものだけを残します（捨てたものは最後に一覧で出ます）
- 事業の説明の文章は持ちません（市の著作物のため。画面では原本のページへリンクします）
- 作られるのは下書きです。[`budget`](#budget) で確かめてから公開します

### budget

```sh
cargo run -- budget list                     # 予算の一覧（自治体・年度・状態）
cargo run -- budget show otawara 2026        # 款別・性質別・歳入・主な事業・捨てたもの
cargo run -- budget publish otawara 2026     # 公開する
cargo run -- budget archive otawara 2026     # 取り下げる
```

### eval

```sh
cargo run -- eval moderation [--model <id>] [--limit <n>] [--file <path>]
cargo run -- eval tagging    [--model <id>] [--limit <n>] [--file <path>]
```

`evals/moderation.jsonl` / `evals/tagging.jsonl` の例で、AI の判定の精度を測ります。**実際の API を呼ぶので費用がかかります。** モデルを変えるときや、指示文を直したときに使います。

## よくある流れ

### 自治体を追加する

1. `data/seeds/010_municipality_<団体コード>_<slug>.sql` を1つ作る（大田原市のファイルに倣う）
   - 自治体・地区・収集先（決算カードは、大田原市の行の id と municipality_id を変えるだけ）を書く
2. `cargo run -- seed`
3. `cargo run -- collect run --source <団体コード>-settlement-card`（決算カードと人口が入る）

### 会議録からトピックを作る

1. 会議録をコピーして `data/private/minutes/` に置く
2. `import minutes ... --dry-run` で、発言者と名簿を確かめる
3. `import minutes ...` で下書きを作る（費用がかかる）
4. `topics drafts` → `topics show <id>` → 必要なら `topics edit` など → `topics publish <id>`

### 今年度の予算を入れる

1. `collect run --source <id> --year <年度> --dry-run`、または手元の PDF で `import budget ... --dry-run`
2. 問題がなければ `--dry-run` を外して下書きを作る（費用がかかる）
3. `budget show <slug> <年度>` で確かめて `budget publish <slug> <年度>`
