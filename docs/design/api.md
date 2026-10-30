# HTTP API

backend（`backend/src/interface/api/`）の API です。すべて `/api` の下にあり、本番ではリバースプロキシが `/api` を backend に振り分けます。
ブラウザは同一オリジンの `/api` を呼び、開発時は `frontend/next.config.ts` の rewrites で backend に転送します。Next.js のサーバー側からは `API_INTERNAL_URL` を呼びます。
型は frontend の `src/lib/api/types/`（backend のモジュールごと）にあります。

## 共通
- エラーは `{"error": {"code", "message"}}`。`message` は画面にそのまま出せる日本語
- **投稿者の情報は、どの API でも返さない**（運営者の API でも。匿名性）
- 自治体に属するデータは、パスの `slug` の自治体で絞る。無効な自治体・別の自治体のものは 404
- 下書き（`topics.status = 'draft'`、予算の draft）は、運営者の API 以外のどこにも出さない
- 参加者は署名付き HttpOnly Cookie `cp_uid` で識別する。**読む・投票はログインなし、投稿はパスキーの登録が要る**（ADR 0006）
- 分析を見られるか・投稿できるかのルールは `backend/src/process/progress.rs`（`Progress::analysis_unlocked`・`post_permission`）の1か所だけ。画面は API が返す `progress`・`posting` を見る（ルールを写さない）

## 自治体・トピック
- `GET /municipalities` / `GET /municipalities/{slug}`（地区つき）— 公開中（`is_active`）の自治体だけ
- `GET /municipalities/{slug}/topics?tag=` / `GET /municipalities/{slug}/topics/{id}`
  - Cookie があれば参加状態 `participation`（`none` / `in_progress` / `completed`）と `voted_count`（承認済みの意見だけで数える）
  - 期限 `closes_at`（null は期限なし）と締め切ったか `closed`。一覧は受付中を先に、それぞれ公開した日（`published_at`。null なら作った日）の新しい順
  - 新着 `is_new`（公開から3日以内で、Cookie の参加者が未投票。`progress::is_new`）、会期 `session`（議会のトピックだけ。出典の最初の会議の資料名と月から「令和8年 第2回定例会（6月）」。`records::minutes::session_label`）
  - 詳細には出典 `sources`（会議録の引用）と `finances`（分野タグに対応する目的別歳出（決算の総額と歳出に占める割合）と、同じ分野の主な事業5件まで（予算））。議会のトピックは会議の年度に合わせる: 予算は会議の日の年（1月だけ前年）の年度でその年度の予算が無ければ出さない、決算はそれより前で最新の年度（`finances::budget_year_of_meeting`）。市民の提案は最新の年度
- `GET /municipalities/{slug}/consensus` — タイムラインにピン留めする合意点（全グループで (A+1)/(S+2) ≥ 0.8）。分析を見られて、1件以上投票したトピックの分だけ（Cookie が無ければ空）

## 参加（投票・投稿）
- `PUT /municipalities/{slug}/relation` — このまちとの関わり方（投票の前に必要）。地区は受け取らない（ADR 0002）
- `GET /municipalities/{slug}/topics/{id}/next-statement` — 未投票の承認済みの意見1件（最初の意見が残っていればその中から。Polis の優先度で重み付けした抽選）、進捗 `progress`（`voted`・`total`・`analysis_unlocked`・`analysis_remaining`）、投稿できるか `posting`（`status`: allowed / registration_required / votes_required（`remaining`）/ limit_reached）
- `POST /municipalities/{slug}/topics/{id}/statements/{sid}/votes` — `{value: 1 | -1 | 0}`。票はあとから変えられない（409 `already_voted`。分析を見てから票を合わせ直すことを防ぐ）
- `POST /municipalities/{slug}/topics/{id}/statements` — `{body}`（1〜140文字）
  - 判定の結果 `status`: approved / rejected / flagged / pending / duplicate（ADR 0007）
  - rejected は理由の種類 `rejection`（harm / person / official_name / not_votable）と言い換えの提案 `suggestion`
  - duplicate は保存せず、案内する意見 `similar`（投票済みならその票 `your_vote`）
  - 保存した意見（rejected 以外）には投稿者の賛成の票を入れる（Polis と同じ）
  - 分析の前は 403 `votes_required`（未登録でも先にこちら）、未登録は 401 `registration_required`、2件目は 409 `statement_limit`
- 締め切ったトピック（`closes_at` を過ぎた。ADR 0005）への投票・投稿は 409 `topic_closed`
- 投稿できる数は1トピック1件。市民提案のトピックを提案した本人は、提案に含めた最初の意見とは別に3件

## トピックの提案（`interface/api/proposals.rs`。ADR 0005）
- `GET /municipalities/{slug}/proposals` — 提案できるか `permission`（`status`: allowed / registration_required / relation_required / weekly_limit（`next_at`））と、本人の提案 `items`（新しい順。`state`: reviewing / published / declined（公開しないまま取り下げ）/ withdrawn（公開のあとに取り下げ））。Cookie が無ければ空
- `POST /municipalities/{slug}/proposals` — `{title, description, statements}`（タイトル1〜60字・説明1〜140字・意見1〜10件で各140字以内。文字数は `shared/limits.rs`）。下書き（`source_type = 'citizen'`）として保存し、意見は承認済みの最初の意見になる。201 で提案1件
  - 未登録は 401 `registration_required`、関わり方が未登録は 409 `relation_required`、その自治体に直近7日で提案していたら 409 `proposal_limit`（見送りになった提案も数える）
  - 入力の誤りは 422 `invalid_proposal`、公人の氏名が入っていたら 422 `official_name`（AI の審査は呼ばない。運営者が全件を見る）
  - 提案した人（`topics.proposed_by`）はどの API にも返さない。提案した人の票は自動で入れない（Polis の seed と同じ。公開のあと、ほかの人と同じく投票する）

## 分析
- `GET /municipalities/{slug}/topics/{id}/analysis` — 意見マップ・グループと代表意見・合意点・`statements_by_group`・`group_composition`（グループごとの関わり方の構成）・関わり方ごとの集計
  - 分析の対象が5人未満なら `insufficient`。関わり方の集計は5人未満の区分を隠し、二次秘匿もする（`suppress`。ADR 0002）
  - **最初の意見すべてに投票するまでは `locked`（残り件数）**。全体の傾向に投票が左右されないようにするため。最初の意見が増えれば（公開の前の下書きでだけ起きる）再びロックされる。締め切ったトピックはロックしない
  - 数理は `docs/polis-math.md`

## まちのお金
- `GET /municipalities/{slug}/finances?year=` — 年度ごとに `settlement`（決算カード）と `budget`（公開済みの当初予算。主な事業つき）
  - 金額は千円、`per_capita` は円。年度を省くと最新（予算だけの年度を含む）
  - `trend` は決算の年度と、決算がまだ無く予算だけがある年度（`kind`: settlement / budget）

## ログイン（パスキー。ADR 0006）
- `GET /auth/me` — `registered`・`operator`（利用者の ID は返さない）
- `POST /auth/register/start` → `finish` — 今の参加者にパスキーを付ける（投票はそのまま）。運営者の招待のトークン `operator_token` を start にも渡すと、パスキーの名前に「運営者」と入る（運営者にするのは finish）
- `POST /auth/login/start` → `finish` — 名前を入力しないパスキー。その端末の匿名の投票をアカウントに移す
- `POST /auth/logout`
- 実装は webauthn-rs（`interface/api/auth.rs`）、アカウントの処理は `records/accounts.rs`。保存するのは公開鍵・ID・日時だけ。設定は `WEBAUTHN_RP_ID`・`WEBAUTHN_ORIGIN`（既定は localhost）。テストはソフトウェアのパスキー（`backend/tests/it/api/auth.rs`）
- 画面の流れ（投票の前の「ログインして投票しますか」・端末の印・アカウントが2つになる限界）は `docs/plans/2026-10-28-phase6a-passkeys.md` の「ログインの流れ」

## 運営者（`/api/operator/...`。`interface/api/operator.rs`）
- 運営者（`users.is_operator`）でなければ、すべて 404（あること自体を見せない）
- パスの自治体のトピック・意見だけを操作できる（準備中の自治体も扱う）
- 中身は CLI と同じ `records`・`process` の関数（ロジックを写さない）
- 一覧は `{items, next}` で分けて返す。`next` を `after` に渡して続きを読む（件数ではなく ID で続けるので、公開・採否で一覧から抜けても読み飛ばさない）。`limit` は50まで

| | |
|---|---|
| `GET /operator/municipalities` | すべての自治体（準備中も） |
| `GET .../{slug}/tags` | 付けられるタグ（分野と、その自治体の地区） |
| `GET .../{slug}/topics?status=draft\|published\|archived&after=&limit=` | トピック（出典・意見と根拠の発言・タグつき）。下書きは古い順、ほかは新しい順 |
| `GET` / `PATCH .../topics/{id}` | 1件（種類 `source_type`: assembly / citizen。提案した人は返さない）/ タイトル・説明の手直し |
| `POST .../topics/{id}/publish` / `archive` | 公開（初めてなら90日の期限が付く）/ 取り下げ（市民の提案なら、本人には「見送り」） |
| `POST` / `DELETE .../topics/{id}/queue` | 公開待ちに入れる（下書きだけ。ほかは 409）/ 外す。公開待ちのものは下書きの一覧（`?status=draft`）に出ない |
| `GET .../{slug}/publishing` | 公開待ち `queue`（公開する順と予定 `planned_at`。時刻が無ければ null）と公開の時刻 `slots` |
| `POST .../{slug}/publish-slots` / `PATCH` / `DELETE .../publish-slots/{id}` | 時刻 `{time_of_day}`（日本時間の HH:MM）を足す・変える・消す（今日の時刻を過ぎていれば明日から）。応答は `publishing` と同じ |
| `PUT .../topics/{id}/closes-at` | `{closes_at?}` 期限の変更。省くと今（すぐ締め切る）。公開中だけ（ほかは 409） |
| `POST .../topics/{id}/statements` | 意見を足す（運営者の意見として承認済み。下書きなら最初の意見） |
| `PUT` / `DELETE .../topics/{id}/tags/{tag_id}` | タグを付ける（運営者のタグとして）/ 外す |
| `GET .../{slug}/statements?after=&limit=` | 確認待ち（flagged）と判定待ち（pending）。種類 `kind`（seed / citizen / added）・理由・根拠の発言 |
| `PATCH .../statements/{id}` | 本文の手直し（投票がある意見は直せない） |
| `POST .../statements/{id}/approve` / `reject` / `retry` | 公開 / 非公開（下書きの意見の取り下げも）/ pending を1件 AI でやり直す（キーが無ければ 409 `llm_unavailable`） |

画面は `/operator`（`?m=<slug>&tab=drafts|scheduled|queue|published`。`frontend/src/components/operator/`）。運営者には、自治体のトップとトピックの詳細のヘッダーに「管理画面」のリンクが出る。

## その他
- `GET /health` — DB 込みの疎通確認
