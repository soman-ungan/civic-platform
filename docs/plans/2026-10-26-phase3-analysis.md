# フェーズ3（分析）実装計画

## Context
フェーズ2で投票ができるようになった。フェーズ3では、投票行列から意見グループと合意点を求め、画面に出す。
- PCA → 2段階クラスタリング → 代表意見（repness）→ 合意点（コンセンサス）
- タイムライン最上位へのピン留め、居住地属性ごとの意見の違い（設計書 §4④）

**ユーザーの方針: ライセンスを本家 Polis と揃え、数理・統計処理は可能な限り Polis の正式なアルゴリズムを採用する。**

調査結果（公式リポジトリ `compdemocracy/polis`、ブランチ `edge`、2026-10-26 時点）:
- **正式な実装**: Clojure 版 `math/src/polismath/math/*.clj` が pol.is 本番で動いてきた基準実装。Python 版 `delphi/` は、これを byte 単位で一致させる目的の移植
- **ライセンス**: ソースのヘッダーは「GNU AGPL, version 3」（**AGPL-3.0-only**。「or later」なし）
- **投票の符号**: Polis 内部では **賛成 = -1、反対 = +1**、パス = 0（`utils.clj` の `agree?`）。このプロジェクトは賛成 = 1 で保存しているので、行列を作るときに反転する
- **既知の癖と不具合**: `delphi/docs/CLOJURE_QUIRKS.md` の Q1〜Q19 と `MATH_ALGORITHM_HISTORY.md`
- **検証用の正解データ**: `delphi/real_data/r4tykwac8thvzv35jrn53-biodiversity/`（投票の CSV と、Clojure 版のコールドスタートでの出力 `*_math_blob_cold_start.json`）。データは Polis openData として **CC BY 4.0** で公開されている

ユーザーと決めたこと:
1. **全体を AGPL-3.0-only にする**（Polis と同じ表記）
2. **合意点のピン留め**: Polis の group-aware consensus を構成する、グループごとの賛成確率 `(A+1)/(S+2)` が**全グループで 0.8 以上**の意見をピン留めする。並び順は group-aware consensus の値（全グループの確率の積）
3. **表示順を Polis 方式に切り替える**: comment-priorities で重みを付けた乱数で選ぶ（`selectProbabilistically`）

完了時の状態: トピック詳細に「みんなの意見」を出す。内容は意見マップ、グループと代表意見、合意点、居住地属性ごとの違い。自治体のタイムライン最上位に合意点をピン留めする。次に出る意見は Polis の優先度で選ばれる。biodiversity データで Clojure 版の出力と一致することを、テストで確かめる。

## 移植の方針（Polis の Clojure 版 → Rust）
| 項目 | 方針 |
|---|---|
| パラメーター | すべて Clojure と同じ: `n-comps=2`、`pca-iters=100`、`base-k=100`、`base-iters=100`、`max-k=5`、`group-k-buffer=4`、in-conv の条件 `min(7, 投票された意見数)` 件以上（足りなければ投票数の多い順に15人まで）、`max-k-fn = min(5, 2 + ⌊基底クラスタ数/12⌋)`、z 値の90%閾値 1.2816 |
| PCA | 列の平均で欠損を補完 → 平均を引く → べき乗法で主成分を2つ（グラム・シュミットで1つ目を除く）→ 参加者はスパース性を考慮して射影（`√(n_cmt/n_vote)` 倍）。意見の射影と extremity（射影の長さ）も出す。前回の主成分を初期値として引き継ぐ |
| クラスタリング | 基底: 射影の上で k-means（k=100。前回のクラスタから clean-start）。グループ: 基底クラスタの中心を、人数で重み付けした k-means（k=2..max-k）→ シルエット係数で k を選ぶ → group-k-smoother（4回連続で同じ k が選ばれたら切り替える。clamp あり） |
| 代表意見 | `comment-stats`（擬似カウント (x+1)/(n+2)、prop-test）→ グループ内とそれ以外の比較（ra / rd、two-prop-test）→ `select-rep-comments`（90%で有意なもの、best-agree を先頭に、repness-metric の上位5件、賛成を先） |
| 合意点 | group-aware consensus（グループごとの確率の積）を計算する。Polis の `select-consensus-comments`（全体で p > 0.5 かつ90%で有意、上位5件）も計算して保存する |
| 表示順 | `priority-metric`: (importance × (1 + 8·2^(−S/5)))²。importance = (1−p)(E+1)a。重み付きの乱数で選び、優先度が無い意見の重みは 1 |
| **取り込む修正** | #2611 の修正（`edge` に入っている）。**Q12**: コールドスタートの初期ベクトルを「すべて1」に固定する（Polis 自身が検証で採る方法で、元の作者のコメントでも乱数のシード固定を TODO にしている）→ 結果が毎回同じになる。**Q2**: 優先度は今回の計算の group-votes から出す（Clojure は前回のものを読む不具合。Python 版の improved モードと同じ） |
| **そのまま再現する仕様** | **Q3**: グループ単位の k-means の反復回数は、実際に動いている 20（`:cluster-iters` が無視されるため）。モデレーションされた意見の列は 0 埋め（`zero-out-columns`）。集計は raw-rating-mat から行う |
| **移植しないもの** | サブグループ（Q7: Polis でも、どこからも使われていない）。大規模な会話向けのミニバッチ PCA（Q10: 規模的に不要。常に通常の PCA を使う）。浮動小数点の癖（Q11 の距離の桁落ち、Q17 のハッシュ順による同点の扱い）。参加者が2人未満・意見が2件未満のときは、計算をせず「データ不足」を返す（Q4 / Q5 / Q14 / Q16 の improved モードと同じ扱い） |

移植したファイルの先頭には、`SPDX-License-Identifier: AGPL-3.0-only`、元のファイルのパスと `edge` のコミット SHA、Polis の著作権表示（`Copyright (C) 2012-present, The Authors`）を書く。一覧は NOTICE にも載せる。

## 進め方
- 実装の前に、この計画を `docs/plans/2026-10-26-phase3-analysis.md` として保存し、最初にコミットする
- `feat/phase3` ブランチで作業し、PR で CI を回してからマージする（スマホでの見た目はユーザーに確認してもらう）
- 新しく追加する依存: `rand`（表示順の重み付き乱数。MIT/Apache-2.0）。線形代数は規模が小さく2成分だけなので、自前で書く（Polis と同じ手順を、そのまま追えるようにするため）

## Step 1: ライセンスを AGPL-3.0-only に → `chore: ライセンス表記を AGPL-3.0-only に変更`
- 全ソースの SPDX ヘッダー、`backend/Cargo.toml`、`frontend/package.json` と lockfile のルート項目、CI のワークフロー
- `NOTICE`（「either version 3 of the License, or (at your option) any later version」→ version 3 のみ）、`README.md`、`CLAUDE.md` のライセンス節、フッターの表記
- `docs/grand-design.md` §7 の表記（AGPL-3.0-or-later → AGPL-3.0-only。理由は Polis と揃えるため）
- `LICENSE` の本文（AGPL v3 の全文）は変えない
- 確認: `git grep -n "or-later\|or later"` で残りが無いこと。CI が成功すること

## Step 2: 数理処理の移植（純粋関数、DB を使わない）→ `feat(analysis): Polis の数理処理を移植`
`backend/src/analysis/` に置く。Clojure の名前空間と1対1で対応させる。
- `stats.rs` ← `stats.clj`（prop-test、two-prop-test、z-sig-90 / 95）
- `matrix.rs` ← `named_matrix.clj` で必要な分（行名・列名付きの行列、欠損は `Option<f64>`、zero-out-columns、rowname-subset。行と列の並びは、最初に投票された順を保つ）
- `pca.rs` ← `pca.clj`（power-iteration、factor-matrix、powerit-pca、wrapped-pca、sparsity-aware-project-ptpt(s)、pca-project-cmnts）
- `clusters.rs` ← `clusters.clj`（init-clusters、cluster-step、weighted-mean、safe-recenter、uniqify / merge、most-distal、clean-start-clusters、kmeans、dist-matrix、silhouette）
- `repness.rs` ← `repness.clj`（comment-stats、add-comparitive-stats、conv-repness、select-rep-comments、consensus-stats、select-consensus-comments）
- `conversation.rs` ← `conversation.clj` の small-conv-update-graph（in-conv、列平均での補完、PCA、射影、基底とグループのクラスタリング、smoother、votes-base / group-votes、group-aware consensus、comment-priorities、repness、consensus）
  - 入力は「前回の状態 + 全投票（user_id, statement_id, 値, 時刻）+ mod-out の意見」。出力は「公開用の結果 + 次回の状態」
- 実装中に読むもの: `named_matrix.clj`（zero-out-columns の意味）と `utils.clj`（`zip`、`mapv-rest`）
- **テスト**
  - 単体: 手で計算した小さな例（prop-test の値、既知の行列での主成分、2つの塊に分かれる k-means、シルエット係数、smoother の切り替え、priority-metric の値）
  - **一致テスト** `tests/polis_parity.rs`: biodiversity の投票を一度に流し（コールドスタート）、`*_math_blob_cold_start.json` と比べる
    - 完全一致: in-conv の集合、選ばれた k、グループごとの参加者集合、グループごとの代表意見の tid と順序、consensus の tid
    - 許容誤差（1e-6）: 主成分（符号は不問）、group-aware consensus、comment-priorities
    - CSV の vote 列の符号（エクスポートは賛成 = 1 の可能性が高い）は、最初に blob の `user-vote-counts` と `votes-base` で突き合わせて確定する
    - 一致しない箇所が残った場合は、無理に合わせない。原因（浮動小数点の癖 Q11 / Q13 / Q17 など）を調べて記録し、ユーザーに報告する
  - 正解データは `backend/tests/fixtures/polis-biodiversity/` に置く（CSV 2.4MB + blob 1.4MB）。出典の表示（CC BY 4.0、The Computational Democracy Project、case study の URL）を `README.md` と NOTICE に書く
- `docs/polis-math.md`: アルゴリズムの説明、パラメーター表、癖ごとの扱い（再現・修正・移植しない）と理由、移植元のコミット SHA、一致テストの結果

## Step 3: 計算結果の保存と定期実行 → `feat(analysis): 分析結果の保存とバックグラウンド計算を追加`
- **新しいマイグレーション** `topic_analyses`（既に main にあるマイグレーションは書き換えない）
  - 列: `topic_id`（PK、FK）、`input_fingerprint`、`result`（公開用の JSON）、`state`（次回の計算に引き継ぐ JSON: 行と列の並び、in-conv、主成分、基底クラスタ、k ごとのグループクラスタ、smoother）、`computed_at`
  - JSON は TEXT に入れる（PG への移行時は jsonb を検討）
- **`analysis::run_topic(pool, topic_id)`**: 投票と mod-out（status が approved 以外の意見）を読む → `conversation::update` → 保存。入力の fingerprint（投票数・最新の投票時刻・意見の status の集計）が前回と同じなら計算しない
- **`analysis::worker`**: `main.rs` から tokio のタスクとして起動する。`ANALYSIS_INTERVAL_SECS`（既定 10 秒）ごとに、fingerprint が変わったトピックを順に計算する（Polis の math worker が投票をポーリングするのと同じ形）。エラーはログに出して次へ進む
- サブコマンド `backend analyze [--topic <id>]`（手動で再計算する。開発用）
- テスト: 2つのグループに分かれる投票を作り、run_topic → 保存された結果を確かめる。fingerprint が同じなら再計算しないこと。2回目は前回の状態を引き継ぐこと（smoother のカウントが進む）

## Step 4: API と表示順 → `feat(backend): 分析結果の API と Polis 方式の表示順を追加`
- `GET /api/municipalities/{slug}/topics/{id}/analysis`
  - データ不足のときは `{status: "insufficient", participant_count, required}`
  - それ以外は `{status: "ready", computed_at, participant_count, groups, consensus, map, residency}`
    - `groups`: `[{id, label("A"…), size, representative: [{statement_id, body, repful_for, n_success, n_trials}]}]`
    - `consensus`: ピン留めの条件（全グループで (A+1)/(S+2) ≥ 0.8）を満たす意見。group-aware consensus の大きい順で、グループごとの賛成率も付ける
    - `map`: 基底クラスタの中心 `{x, y, count, group_id}` と、意見の射影。個人の点は出さない。Cookie がある人には、保存済みの主成分から自分の位置を計算して `you` で返す
    - `residency`: 居住地属性ごとの、意見ごとの賛成率。**1つのセルが5人未満なら出さない**（個人の特定を防ぐため）
- `GET /api/municipalities/{slug}/consensus` → 自治体内の全トピックの合意点（トピック名つき）。タイムラインのピン留めに使う
- `next-statement` を変える: 未投票の承認済み意見を候補にし、保存済みの comment-priorities で重みを付けた乱数で1件選ぶ（`selectProbabilistically` と同じ。優先度の無い意見の重みは 1、分析が無ければ一様）。乱数の部分を切り出した純粋関数にし、テストでは乱数を固定する
- テスト: 自治体の取り違えは 404。`user_id` が含まれないこと。5人未満のセルが隠れること。`you` は Cookie があるときだけ返ること。ピン留めの条件。重み付きの選び方

## Step 5: フロントエンド → `feat(frontend): 意見マップ・グループ・合意点の表示を追加`
書く前に `node_modules/next/dist/docs/` を確認する（AGENTS.md）。
- トピック詳細の投票画面の下に「みんなの意見」の節を置く（Polis の参加画面と同じく、投票しながら分析も見られる配置にする。投票の有無で表示は制限しない）
- 意見マップ: SVG（新しい依存は入れない）。基底クラスタを人数に応じた大きさの点で描き、グループごとに色分け。「あなた」の位置を強調。スマホ幅に収める
- グループのカード: 「グループA（n人）」と代表意見。「このグループの n_success/n_trials（%）が賛成／反対」と表示
- 合意点: 「全グループの8割以上が賛成」の意見
- 住民と関係人口の違い: 賛成率の差が大きい意見を表で出す（隠したセルは「—」）
- 自治体のタイムライン: 一覧の上に「合意点」の枠を出し、その意見とトピックへのリンクを並べる
- データ不足のときは「分析には、もう少し参加者が必要です（n / 必要数）」

## Step 6: ドキュメント → `docs: フェーズ3の分析と Polis 由来の処理を記録`
- CLAUDE.md: 分析の API、`ANALYSIS_INTERVAL_SECS`、`backend analyze`、Polis 由来のコードの扱い（`docs/polis-math.md` を参照）
- 実装計画に「実装時の変更点」を追記する

## 動作確認
1. `cargo test`: 単体テスト、一致テスト（biodiversity）、API のテストがすべて成功。clippy の警告なし。CI の3ジョブが成功（PG の互換チェックに新しいマイグレーションも含む）
2. 一致テストの結果（どの項目が完全一致か、許容誤差内か）を `docs/polis-math.md` に記録する
3. `seed --dev` → curl で20人ほどの投票を2つの意見傾向に分けて入れる → `backend analyze` → API で2グループ・代表意見・合意点が出ることを確かめる
4. 画面: 意見マップ・グループ・合意点・タイムラインのピン留めが表示される（スマホ幅の確認はユーザーにお願いする）
5. `git grep -n "or-later"` が0件。NOTICE に Polis 由来のファイルと biodiversity データの出典がある

## 実装時の変更点

計画から外れた点と、その理由。

- **一致テストの合否基準**: 計画では「代表意見の tid と順序は完全一致」などを求めていた。正解データの作り方を調べると、Polis 自身の比較テスト（`delphi/tests/test_legacy_clojure_regression.py`）は、主成分を角度 10° 以内、グループを Jaccard 0.99 以上・人数分布の差 0.01 以内で比べていた。そこで基準をこれに合わせた。結果はそれより良く、代表意見と全体の合意点は完全に一致した（`docs/polis-math.md`）。
- **一致テストでの行と列の並び**: 正解データの列は意見 ID の昇順だった。統計値がまったく同じ意見の並び順は列の順で決まるため、テストでは行と列を ID の昇順に並べてから計算している。行の並びは結果に影響しないことも確かめた。
- **グループの所属が1人だけ異なる**: 基底クラスタ（100個）の段階で、同点の扱い（Q11 の距離の桁落ちと Q17 の Clojure のハッシュ順）を再現していないため。計画どおり再現はしない。group-aware consensus の差（最大 6.8e-3）もこの1人によるもので、テストでは 0.01 未満を許容している。
- **優先度は正解データと比べていない**: 正解データ（2025-11 生成）は #1961 の不具合で優先度がすべて 49 になっていた。式は単体テストで確かめた。
- **Polis との違いを2点追加**: clean-start で、メンバーが今回のデータにいないクラスタの中心は前回の値を保つ（Clojure では NaN）。シルエット係数で比べる相手のクラスタが無いときは 0 とする（Clojure では例外）。
- **グループを表示する最小人数（5人）**: 計画にはなかった。少人数だとグループの集計が個人の投票そのものになるため、分析対象が5人未満のうちは insufficient を返す（居住地属性のセルを5人未満で隠すのと同じ考え方）。Polis にはない、このプロジェクトの判断。
- **best-agree の表示**: 全員が同じ投票をした意見が best-agree に選ばれると、Polis の finalize では反対側に分類され、「0人が反対」と表示されてしまった。Polis の `:n-agree` と同じく賛成数を持たせ、API では賛成として返すようにした（動作確認中に発見）。
- **データ不足のあとに再計算できない不具合**: 最初の計算がデータ不足だったトピックで、保存した状態（`{}`）が読み込めず、その後の計算が失敗していた。`ConvState` に `serde(default)` を付けて修正し、テストを追加した（`analyze --force` の確認中に発見）。
- **新しいマイグレーションを追加**: 計画どおり、main にマージ済みのマイグレーションは書き換えず、`topic_analyses` を新しいファイルで追加した。ライセンス変更（Step 1）では初回マイグレーションのヘッダーを書き換えたので、手元の DB は作り直しが必要になった。今後は書き換えないよう CLAUDE.md にルールを追記した。
- **スマホ幅での表示確認**: この環境にブラウザがないため、API と HTML は curl で確認した。見た目はユーザーに確認してもらう。
- **開発用サンプルに参加者と投票を追加**（ユーザーの確認で判明）: サンプルには意見しか無く、分析対象が足りないため「最新の分析を見る」で分析が出なかった。架空の参加者40人と、トピックごとに意見の傾向が分かれる投票を `data/dev-seeds/110_sample_votes_092100_otawara.sql` に追加した（`scripts/generate_sample_votes.py` で生成、シード固定）。DB を初期化しても `seed --dev` で同じ状態になる。サンプルだけで全トピックの分析が ready になり、合意点が出ることを `tests/seeds.rs` で確かめている。
  - 最初はばらつき（逆の票 8%、投票しない 7%）を強くしすぎた。逆の票が重なった少数の人が独立したグループになり、合意点が出なかったため、3% ずつに下げた。

## 追加の要望（2026-10-26、レビュー時）

- **参加状態の表示**: トピック一覧・詳細の API に、Cookie の参加者ごとの `participation`（未参加 / 参加中 / 投票済み）と `voted_count` を追加。承認済みの意見だけで数えるので、意見が増えると「投票済み」は「参加中」に戻る。一覧はサーバー側で描画するため、ブラウザの Cookie ヘッダーを backend に転送して、最初の表示から正しい状態にする。カードは未参加＝白、参加中＝琥珀色の縁取りと「参加中 n/m」、投票済み＝淡い緑と「✓ 投票済み」（色だけに頼らず文字でも示す）。
- **合意点の「もっと見る」**: タイムラインの「みんなの合意点」は最初の3件だけを表示し、「もっと見る（残り n 件）」で開く。API の上限は 5 件から 20 件に広げた。
- **グループごとの特徴をテーブルに**: 行 = 承認済みの意見（投稿順）、列 = グループにして、各グループの賛成・反対の割合と内訳の帯、投票人数を一覧する。分析 API に `statements_by_group`（Polis の group-votes）を追加。スマホでは表を横にスクロールでき、意見の列は左端に固定する。自分のグループの列は見出しに「あなた」のバッジを付けて列全体を色付けする。代表意見（repness）に選ばれたセルには ★ を付け、これまでのグループごとのカードは表に置き換えた。
