# フェーズ6d: 市民からのトピックの提案

## Context
ADR 0005 は、市民からのトピックを「提案」として受け、運営者の確認を通ったものだけを公開すると決めている（1人週1件。提案した本人は、そのトピックに意見を3件まで）。
いまトピックは会議録から作る議会のトピック（`source_type = 'assembly'`）だけで、`source_type = 'citizen'` は CHECK にあるが使っていない。管理画面（6c）には下書きの確認と公開がすでにある。

## 決めたこと（2026-10-29 ユーザーと決定）
- **提案は「タイトル・説明・意見1〜3件」**。意見は公開すると最初の意見（`is_seed`）になる。提案した本人は、そのトピックに合わせて3件まで書ける（公開後は残りの分だけ）
- **説明は1〜140字で、本人が書く**（議会のトピックの AI の要約と同じ長さ。`MAX_DESCRIPTION_CHARS` を共通に使う）。AI でまとめ直さない（本人の言葉を言い換えない・キーが無くても動く・費用がかからない）。主張は説明でなく意見に書いてもらい、中立さは運営者が公開の前に直す
  - 比べた案: AI が140字にまとめる（キーが要る・本人の言葉が変わる）、長いまま（市民トピックだけ長くなり、投票できない形で主張が残る）
- **結果は本人だけが「自分の提案」の一覧で見る**（確認中・公開・見送り。見送りの理由は出さない）
- **提出のときは公人の氏名だけをプログラムで確かめる**（AI の審査は呼ばない。運営者が全件を見るため）
- **提案は自治体ごとに1人週1件**（直近7日。見送りになった提案も数える）

## やること

### Step 1 DB
- 新しいマイグレーション: `topics.proposed_by TEXT REFERENCES users (id)`（市民の提案だけに入る）と、`(proposed_by, municipality_id, created_at)` の索引
- `proposed_by` は API のどこにも返さない（運営者の API でも。運営者に見えるのは `source_type = 'citizen'` だけ）

### Step 2 ルール（`process/progress.rs`）
- `propose_permission`: パスキーの登録と、その自治体との関わり方の登録が要る。その自治体に直近7日で1件提案していたら `weekly_limit`
- `post_permission` の上限: そのトピックの提案者なら 3（`STATEMENTS_PER_TOPIC_PROPOSER`）、ほかの人は 1

### Step 3 提出と結果の API（`records/proposals.rs`・`interface/api/proposals.rs`）
- `POST /municipalities/{slug}/proposals` — `{title, description, statements[1..3]}`。タイトル1〜60字・説明1〜140字・意見1〜140字。公人の氏名が入っていたら 422 で書き直してもらう。保存は `draft`・`citizen`、意見は `approved`・`is_seed`・投稿者は本人
- `GET /municipalities/{slug}/proposals` — 本人の提案だけと、提案できるか `permission`。`state`: reviewing / published（トピックの ID）/ declined（公開しないまま取り下げた）/ withdrawn（公開後に取り下げた）
- 本人の賛成の票は**公開のときに入れる**（提出のときに入れると、運営者が意見を直せなくなるため。「投票がある意見は直せない」）

### Step 4 管理画面・CLI
- 下書きのタブに市民の提案も並べ、「市民の提案」の印を付ける。手直し・タグ・公開・取り下げ（＝見送り）は今の処理をそのまま使う
- CLI の `list topic` にも種類を出す

### Step 5 画面
- 自治体のトップ（タイムライン）に「トピックを提案する」の入口を足し、`/[municipality]/proposals` に提案のフォームと「自分の提案」の一覧を置く
- フォームの案内: 説明は「何が課題か、どんな選択肢があるか」。あなたの考えは意見に
- 出せない理由（未登録・週1件・関わり方が未登録）は API の `permission` をそのまま表示する（ルールを写さない）

### Step 6 テストと docs
- `tests/it/api/proposals.rs`（2自治体で。週1件が自治体ごと・3件の上限・`proposed_by` が API に出ないこと・公開のときの賛成の票・下書きが外に出ないこと・公人の氏名）
- ADR 0005 の追記、`design/schema.md`・`design/api.md`・`operations/data-flow.md`・`operations/cli.md`・`overview.md`・`plans/README.md`・CLAUDE.md のフェーズ

## あとで決めること
- 同じ論点のトピックがすでにある提案は、今回は運営者が見送るだけにする（似たトピックの判定は、意見の取りまとめのフェーズで）
- タイトルの上限（60字）は仮の値

## 実装時の変更点
- 文字数の上限（タイトル60字・説明140字・意見140字）を `backend/src/shared/limits.rs` にまとめ、会議録の取り込み・意見の投稿・提案で同じ値を使うようにした（前は `ingest/minutes/extract.rs` と `interface/api/participation.rs` に別々にあった）。frontend は `src/lib/statement.ts`
- 公人の氏名の確かめは、意見の審査と同じ関数（`process::moderation::official_name_in`）を API から呼ぶ。`records` は `process` を使わないので、確かめは `interface/api/proposals.rs` に置いた
- 本人の賛成の票は `records::topics::set_status`（公開）の中で入れる。CLI の `publish topic` と管理画面の公開の両方に効く
- 投稿の上限に達したときの文（API と画面）を「1トピック1件まで」から「投稿できる意見の数に達しました」にした（提案した本人は3件のため）
- 関わり方のシート（`RelationSheet`）に保存のボタンの文を渡せるようにし、提案の画面では「保存して提案する」にした
- 提案の画面では、公開したあとに自動のタグ付けでタイトルと説明を AI に送ることを伝える
- 確かめ: `cargo test`（新しいテスト6件を含め、すべて通る）・clippy・frontend の lint と型チェック。コピーした DB で backend と frontend を起動し、API（提出・週1件）と画面の HTML（未登録・週1件・フォーム・タイムラインの入口）を確かめた。ブラウザが無いので、パスキーの登録と見た目は手元で確かめてもらう
- 2026-10-30 ユーザーと決定: 提案した人の賛成の票を公開のときに入れるのをやめた。Polis 本家（`server/src/routes/comments.ts`）は、主催者が入れる最初の意見（seed）に自動の票を付けず、主催者もほかの人と同じく投票する。これに合わせ、提案した人が反対の立場の意見も最初の意見に書けるようにした。参加者の投稿は Polis と同じく賛成に固定のまま（立場を選べると、わざと弱く書いた相手の立場の意見（わら人形）を出せるため）。管理画面の確かめる点に、わら人形を足した
- 提案のフォームに、3件はこのトピック全体の数で、提案に書いた分も数えることと、公開のあとに出せる残りの件数を表示した（3件をすべて提案で使えると誤解されないように）
