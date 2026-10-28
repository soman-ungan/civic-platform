# backend のコマンド一覧（運営者向け）

backend（Rust）は、API サーバーとしてだけでなく、運営のためのコマンドとしても使います。
自治体のデータの投入、会議録・予算の取り込み、AI の判定の確認などは、すべてこのコマンドで行います。
管理画面は、認証（開発フェーズ6）ができるまではありません。
資料がどのコマンドで DB のどこに入るかの全体は、[docs/data-flow.md](data-flow.md) にあります。

- 開発中は `backend/` で `cargo run -- <コマンド>` として実行します
- ビルドしたものは `./backend <コマンド>`（`cargo build --release` なら `target/release/backend`）でも同じです
- 引数を付けずに実行すると、API サーバーが起動します
- どのコマンドも、起動のたびに DB のマイグレーションを自動で適用します
- `--help` で、コマンドと引数の一覧が出ます（例: `cargo run -- --help`、`cargo run -- import --help`）。`--help` は設定（`.env`）や DB が無くても使えます

## コマンドの形

```
cargo run -- <動詞> <対象> [オプション]
```

- **動詞は段階ごとに分かれています**（下の表）。対象は、その段階で扱えるものだけを選べます（`fetch topic` などはエラー）
- **その対象には要らない処理は、何もせずに通します。** 例: `extract settlement`（決算カードは AI を使わない）は「何もしません」と出して、終了コード 0 で終わります。資料の種類によらず、同じ手順を書けます
- **どれを指すかは、すべてオプションで渡します。** 名前はどの動詞でも同じです

| オプション | 意味 |
|---|---|
| `-m`, `--municipality <slug>` | 自治体の URL の名前（例: `otawara`） |
| `-y`, `--year <n>` | 年度（西暦） |
| `--id <id>` | トピック・意見・会議録（document）・収集先・タグの id |
| `--topic <id>` | 対象のトピック（意見を足す・タグを付けるときなど） |
| `--file <path>` | 原本のファイル |
| `--dry-run` | 確かめるだけで、保存しない |
| `--again` / `--force` | 作り直す / 形式的な会議録も取り込む |

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
| `ANTHROPIC_API_KEY`（Claude API） | `extract`（`--manual`・`--dry-run` を除く）・`moderate`・`classify`・`eval` | AI を使わない。投稿は自動で承認され、タグ付けはしない。AI が要るコマンドはエラーになる |
| `COLLECTOR_CONTACT`（運営者の連絡先） | `fetch` | 取得しない（取得先の User-Agent に入れるため） |
| poppler-utils（`pdftotext`） | 予算の `extract`・`import` | 予算を取り込めない（`apt install poppler-utils`） |
| `PRIVATE_DIR`（既定 `../data/private`） | `fetch`・`import` | 既定の場所を使う |

### 費用がかかるコマンド

Claude API を呼ぶのは、段階1では **`extract` だけ**です（`fetch` と `import` は呼びません）。料金は Anthropic の API の料金で、目安は次のとおりです。

| コマンド | 目安 |
|---|---|
| `extract minutes`（会議録1日分） | 0.3〜0.4 ドル（`--manual`・`--dry-run` なら費用なし） |
| `extract budget`（当初予算の概要、40ページ） | 約 0.8 ドル（`--manual`・`--dry-run` なら費用なし） |
| `eval moderation` / `eval tagging` | 件数による |
| `moderate statement`・`classify topic`・サーバーのモデレーションとタグ付け | 投稿・トピックの数による（`llm_usage` で確かめられる） |

1日（UTC）の費用には上限があり、超えると AI を呼びません。

| 環境変数 | 既定 | 対象 |
|---|---|---|
| `LLM_DAILY_BUDGET_USD` | 1.0 | モデレーション・タグ付け・評価 |
| `LLM_EXTRACTION_DAILY_BUDGET_USD` | 5.0 | 会議録・予算の `extract`（別枠） |

使った量と推定の費用は、DB の `llm_usage` に記録されます。

## コマンドの一覧

「無」は何もせずに通す組み合わせです。

### 1. 資料を DB に入れる（fetch → extract → import）

| | `settlement`（決算カード） | `budget`（当初予算の概要） | `minutes`（会議録） |
|---|---|---|---|
| [`fetch`](#fetch) | 総務省から取得 | 市のサイトから取得 | 無（規約のため、手でコピー） |
| [`extract`](#extract) | 無（AI を使わない） | AI で読み、JSON に書く | 同じ |
| [`import`](#import) | Excel を読んで保存 | JSON を検査し、下書きに保存 | 同じ |

### 2. DB を見る・直す

| | `budget` | `settlement` | `minutes` | `topic` | `statement` | `tag` | `source` | `operator` |
|---|---|---|---|---|---|---|---|---|
| [`list`](#list) | ○ | ○ | ○ | ○ | ○ | ○ | ○ | ○ |
| [`show`](#show) | ○ | ○ | ○ | ○ | ○ | 無 | ○ | 無 |
| [`edit`](#edit--add--remove) | 無 | 無 | 無 | ○ | ○ | 無 | 無 | 無 |
| [`add` / `remove`](#edit--add--remove) | 無 | 無 | 無 | 無 | ○ | ○ | 無 | ○ |
| [`publish` / `archive`](#publish--archive) | ○ | 無 | 無 | ○ | 無（`moderate` で） | 無 | 無 | 無 |

### 3. DB の中身を判定・計算する

API サーバーが裏で自動で動かしている処理を、今すぐ動かします。

| | `topic` | `statement` |
|---|---|---|
| [`moderate`](#moderate) | 無（市民からのトピックの提案ができたら） | 判定待ちを AI で判定し直す・人が承認／非公開にする |
| [`classify`](#classify) | AI で分野・地区のタグを付ける | 無 |
| [`analyze`](#analyze) | 意見マップ・グループ・合意点を計算する | 無 |

### 準備と保守

| コマンド | すること |
|---|---|
| （引数なし） | API サーバーを起動する |
| [`seed`](#seed) | マスタ（自治体・地区・分野タグ・公人の名簿・収集先）を入れる |
| [`eval`](#eval) | AI のモデレーション・タグ付けの精度を評価する |

---

## 1. 資料を DB に入れる

3つのコマンドは、原本の隣に置く記録（`<名前>.meta.json`）でつながっています。

```
fetch                         extract                          import
原本を取得して置き場に置く  →  AI で読み、出力を JSON に書く  →  検査して DB に入れ、置き場の原本を消す
<年度>.pdf                     <名前>.extraction.json            （手でコピーした原本は消さない）
<年度>.meta.json（URL・取得の記録）  meta に作り方（api / manual）を足す
```

- `fetch` で取った原本は `data/private/fetched/<slug>/<対象>/`（git の管理外）に置きます。`import` が保存を終えたら消します（`--keep` で残す）。原本は保持しない方針です（`docs/design/collection.md`）
- 手でコピーした原本（会議録・手で取った PDF）は、`data/private/` に置いて `--file` で渡します。`import` でも消えません
- 変わっていない資料は取り直しません（取得の記録は、`import` が保存を終えたときに残します。失敗したら次の `fetch` で取り直します）

### fetch

```sh
cargo run -- fetch settlement -m otawara              # 決算カード（平成27年度以降すべて）
cargo run -- fetch settlement -m otawara -y 2023      # 1つの年度だけ
cargo run -- fetch budget -m otawara -y 2026          # 当初予算の概要の PDF（AI は呼ばない）
cargo run -- fetch settlement                         # 有効な収集先すべて
cargo run -- fetch budget --source 092100-budget-overview --dry-run   # 取得して確かめるだけ
```

| オプション | 意味 |
|---|---|
| `-m <slug>` | その自治体の収集先だけ（無ければ、その資料の有効な収集先すべて） |
| `-y <n>` | 1つの年度だけ |
| `--source <id>` | 1つの収集先だけ（`list source` の id） |
| `--dry-run` | 取得して確かめるだけで、置き場に置かない |

- 収集先は `collector_sources` の行です。自治体の seed ファイルに書きます（`data/seeds/010_municipality_092100_otawara.sql` を参照）
- 取得は、robots.txt に従い、同じサイトには5秒以上あけ、変わっていない資料は取り直しません
- 利用規約の確認の記録（`terms_url`・`terms_checked_at`）が無い収集先は動きません。確認から1年を過ぎると警告が出ます（`docs/design/collection.md`）
- 総務省の決算カードは、全国の市町村で同じ設定で取得できます（平成27年度以降）。どの Excel にその自治体のシートがあるかまでを `fetch` が見て、1年度に1つだけ置きます
- 公開済みの年度の予算の概要が更新されていても、取得しません（警告が出ます。作り直すときは手で取って `import budget --again`）
- `fetch minutes` は何もしません（会議録は規約のため、手でコピーします）

### extract

```sh
# 予算（既定は置き場の <年度>.pdf。手で取った PDF は --file）
cargo run -- extract budget -m otawara -y 2026                  # API で読む（費用がかかる）
cargo run -- extract budget -m otawara -y 2026 --manual         # 共同作業の指示文を書き出す（費用なし）
cargo run -- extract budget -m otawara -y 2026 --dry-run        # ページ数を確かめるだけ（費用なし）

# 会議録
cargo run -- extract minutes -m otawara \
  --file ../data/private/minutes/2026R08_0226.txt \
  --title "令和8年第1回定例会（第2号）" --held-on 2026-02-26 --url "https://..." --dry-run
```

| オプション | 意味 |
|---|---|
| `-m <slug>` | 自治体 |
| `-y <n>` | 年度（予算） |
| `--file <path>` | 原本（予算は既定で置き場の PDF。会議録は必須。UTF-8 のテキストを `data/private/minutes/` に置く） |
| `--title` / `--held-on` | 会議名と開催日（`YYYY-MM-DD`。会議録）。記録に残り、`import` では打たなくてよい |
| `--url` | 原本のページ（画面の出典のリンク）。記録に残る |
| `--manual` | API を呼ばず、共同作業の指示文・出力の形・入力を `<名前>.prompt.md` に書き出す（[共同作業で取り込む](#共同作業で取り込むapi-を使わない)） |
| `--dry-run` | AI を呼ばずに入力を確かめる。会議録は、発言者・名簿に無い首長と議員・議事日程を出す |
| `--out <path>` | 指示文の書き出す先（`--manual`） |

- **`--manual`・`--dry-run` を付けないと API を呼び、費用がかかります**（会議録1日分 0.3〜0.4 ドル、予算 40ページ約 0.8 ドル）
- 出力は原本の隣の `<名前>.extraction.json` に書き、検査の結果も出します（保存はしません）
- `extract settlement` は何もしません（決算カードは AI を使わず、`import` が Excel を読みます）

### import

```sh
cargo run -- import settlement -m otawara                       # 置き場の決算カードを保存し、原本を消す
cargo run -- import budget -m otawara -y 2026 --dry-run         # 検査だけ
cargo run -- import budget -m otawara -y 2026                   # 下書きに保存し、置き場の原本を消す
cargo run -- import minutes -m otawara --file ../data/private/minutes/2026R08_0226.txt --dry-run
```

| オプション | 意味 |
|---|---|
| `-m <slug>` | 自治体（決算カードは、無ければ置き場のすべての自治体） |
| `-y <n>` | 年度（予算は必須。決算カードは1つの年度だけにする） |
| `--file <path>` | 原本（予算は既定で置き場の PDF。会議録は必須） |
| `--extraction <json>` | AI の出力（既定は原本の隣の `<名前>.extraction.json`）。別の場所から渡したものは共同作業の出力とみなす |
| `--title` / `--held-on` / `--url` | 既定は `fetch`・`extract` のときの値 |
| `--dry-run` | 検査の結果を出すだけで、保存しない |
| `--again` | 取り込み済みの会議録・公開済みの予算を作り直す（予算は下書きに戻る） |
| `--force` | 一般質問・代表質問の無い会議録（採決だけの日など）も取り込む |
| `--keep` | 取り込んだあとも、置き場の原本を消さない |

- **API は呼びません。** AI の出力が無ければ止まります（先に `extract`）。会議録の最初の意見はモデレーションに通ります（API キーがあれば AI が判定します）
- 予算: AI が読んだ款別・性質別・歳入の額は、合計が合わなければ保存しません。主な事業は、名前と金額がそのページの本文にあるものだけを残します（捨てたものは最後に一覧で出ます）。事業の説明の文章は持ちません（市の著作物のため。画面では原本のページへリンクします）
- 会議録: AI には、その自治体の公開中・下書きのトピックの一覧も渡します。同じ論点（何を決める・変えるかが同じ）のトピックには印（`same_as`）が付き、新しいトピックにはせず意見を足します。下書きには最初の意見として、公開中のトピックには確認待ち（`list statement` → `moderate statement --id <id> --approve | --reject`）で足します（ADR 0005）
- 会議録: 最初の意見が3件未満のトピックも捨てずに下書きにし、「意見が少ない」と知らせます。立場が偏っていないかを `show topic` で確かめてください
- 会議録: 名簿に無い首長・議員がいると止まります。`data/seeds/030_officials_<団体コード>_<slug>.sql` に登録して `seed` してから、やり直してください
- 予算と会議録で作られるのは下書きです。段階2の `show` で確かめてから `publish` します（`docs/adr/0004-assembly-minutes.md`）。決算カードは、取り込んだら公開されます
- 置き場に新しい原本が無ければ（変わっていなかったときなど）、何もせずに通します

## 2. DB を見る・直す

下書きは、API のどこにも出ません。ここで確かめてから公開します。

### list

```sh
cargo run -- list budget [-m otawara]          # 予算（自治体・年度・状態・歳出）
cargo run -- list settlement [-m otawara]      # 決算カード（年度・歳出・人口）
cargo run -- list minutes [-m otawara]         # 取り込んだ会議録（作ったトピックの数・作り方）
cargo run -- list topic [-m otawara]           # トピック（状態つき）
cargo run -- list topic --drafts               # 下書きのトピックだけ
cargo run -- list statement                    # 確認待ち（flagged）と判定待ち（pending）の意見
cargo run -- list statement --status approved --topic <id>
cargo run -- list tag -m otawara               # 分野の語彙と、その自治体の地区（add tag に使う id）
cargo run -- list source                       # 収集先・規約の確認日・最後の取得
cargo run -- list operator                     # 運営者のアカウント（ID・登録日・パスキーの数）
```

### show

```sh
cargo run -- show budget -m otawara -y 2026        # 款別・性質別・歳入・主な事業・捨てたもの
cargo run -- show settlement -m otawara -y 2023    # 費目ごとの金額と人口
cargo run -- show minutes --id <document の id>    # 会議録と、そこから作ったトピック
cargo run -- show topic --id <id>                  # 説明・出典の引用・最初の意見・タグ
cargo run -- show statement --id <id>              # 本文・状態・判定の理由・投票の数
cargo run -- show source --id <id>                 # 収集先の設定
```

意見の投稿者が誰かは、運営のコマンドでも出しません（最初の意見か市民の投稿かだけ）。

### edit / add / remove

```sh
cargo run -- edit topic --id <id> --title "..." --description "..."
cargo run -- edit statement --id <意見の id> --body "直した本文"
cargo run -- add statement --topic <topic_id> --body "意見の本文"
cargo run -- remove statement --id <意見の id>
cargo run -- add tag --topic <topic_id> --id field-kosodate      # 地区は 092100-kurobane など
cargo run -- remove tag --topic <topic_id> --id field-kosodate
cargo run -- add operator                                         # 運営者の招待のリンク（24時間・1回だけ）
cargo run -- remove operator --id <アカウントの id>               # 運営者から外す（アカウントは残る）
```

- 運営者は画面からは増やせません（ADR 0006）。`add operator` が出すリンク（`<WEBAUTHN_ORIGIN>/operator/join?token=…`）を本人にだけ渡し、そこでパスキーを登録したアカウントが運営者になります。DB にはトークンのハッシュだけを残します

- 投票がある意見は、直せません（投票の意味が変わってしまうため）
- 意見は140文字以内です
- 資料から取り込んだ数字・原文（予算・決算・会議録）は手で直しません（`import --again` で作り直す）。タグの語彙と収集先は seed で管理します

### publish / archive

```sh
cargo run -- publish topic --id <id>              # 公開する
cargo run -- archive topic --id <id>              # 取り下げる（画面から消える。データは残る）
cargo run -- publish budget -m otawara -y 2026
cargo run -- archive budget -m otawara -y 2026
```

- 会議録から作ったトピックは、1つずつ公開します（`show minutes` で一覧）
- 意見の公開・非公開は、判定として [`moderate`](#moderate) で行います

## 3. DB の中身を判定・計算する

### moderate

```sh
cargo run -- moderate statement                        # 判定待ち（pending）を AI でもう一度判定する
cargo run -- moderate statement --id <id> --approve    # 人が公開する
cargo run -- moderate statement --id <id> --reject     # 人が非公開にする
```

- 確認待ちの一覧は `list statement` です
- AI は、主張や政治的な立場では判断しません。公人の公務への批判は公開し、人格攻撃や私生活に触れるものは非公開にします（`docs/adr/0003-llm-operations.md`）
- 通信の失敗や費用の上限で判定できなかった意見は、自動で承認せず pending のまま残します

### classify

```sh
cargo run -- classify topic                       # タグの無いトピックすべて
cargo run -- classify topic --id <id>             # 1つのトピック
cargo run -- classify topic --id <id> --force     # AI が付けたタグだけを付け直す（運営者が付けたタグは残す）
```

サーバーは60秒ごとに、タグの無いトピックを自動で処理します。分野の語彙と判断の基準は `data/seeds/020_field_tags.sql` と `docs/design/tagging-criteria.md` にあります。手で付けるときは `add tag` です。

### analyze

```sh
cargo run -- analyze topic                  # 投票が変わったトピックだけ
cargo run -- analyze topic --force          # すべての公開済みのトピック
cargo run -- analyze topic --id <id>        # 1つのトピック
```

サーバーは `ANALYSIS_INTERVAL_SECS`（既定10秒）ごとに自動で計算します。このコマンドは、すぐに確かめたいときに使います。

## 準備と保守

### seed

```sh
cargo run -- seed          # data/seeds/*.sql をファイル名の順に入れる
cargo run -- seed --dev    # 続けて data/dev-seeds/（【サンプル】の架空のデータ）も入れる
```

- 何度実行しても同じ結果になります（すでにある行は入れ直しません。名簿などは更新します）
- `--dev` は開発用です。架空の参加者120人と投票が入るので、分析の画面をすぐに確かめられます。**本番では使いません**

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
3. `cargo run -- fetch settlement -m <slug>` → `cargo run -- import settlement -m <slug>`（決算カードと人口が入る）

### 決算カードを入れる（年1回）

```sh
cargo run -- fetch settlement -m otawara
cargo run -- import settlement -m otawara      # 原本はここで消える
cargo run -- list settlement -m otawara
```

### 今年度の予算を入れる

1. `fetch budget -m <slug> -y <年度>`（または、手で取った PDF を `--file` で渡す）
2. `extract budget -m <slug> -y <年度>`（費用がかかる）か、共同作業なら `--manual`
3. `import budget -m <slug> -y <年度> --dry-run` で確かめ、`--dry-run` を外して下書きを作る
4. `show budget -m <slug> -y <年度>` で確かめて `publish budget -m <slug> -y <年度>`

### 会議録からトピックを作る

1. 会議録をコピーして `data/private/minutes/` に置く
2. `extract minutes ... --dry-run` で、発言者と名簿を確かめる
3. `extract minutes ...`（費用がかかる）か、共同作業なら `--manual`
4. `import minutes -m <slug> --file <パス>` で下書きを作る
5. `list topic --drafts` → `show topic --id <id>` → 必要なら `edit topic` など → `publish topic --id <id>`

### 共同作業で取り込む（API を使わない）

会議録・予算の概要の取り込みは、年に数回です。API の費用をかけずに、Claude Code との共同作業でも取り込めます。出力は API の時と同じ検査を通ります。手順は `.claude/commands/import-minutes.md`・`import-budget.md` にあり、Claude Code では `/import-minutes`・`/import-budget` で始められます。

1. `extract ... --manual` で指示文・出力の形・入力を書き出す
2. Claude Code が原本と指示文を読み、JSON（原本の隣の `<名前>.extraction.json`）を作る。運営者はその場で原本と見比べる
3. `import ... --dry-run` で検査に通す（捨てたものを見て直す）
4. `import ...` で下書きを保存し、`show topic` / `show budget` で確かめて公開する

- 本番のサーバーで取り込むときは、手元で作った JSON と原本をサーバーに送り、サーバーで 3・4 を行います。サーバーに API キーが無くても取り込めます（キーは投稿のモデレーションのため）
- API キーが無いと、トピックの自動のタグ付けは動きません。`add tag` で付けます
- どちらの方法で作ったかは、`documents.extraction`（`api` / `manual`）に残ります
