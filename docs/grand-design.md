### 【グランドデザイン v2】地方行政可視化＆市民参加プラットフォーム（Polis思想準拠・多自治体対応）

> v2 変更点：対象を大田原市に固定せず、行政区分（都道府県 → 市区町村 → 地区）を選択可能な構造に変更。大田原市を最初の対象自治体（パイロット）として扱う。
> v2.1 変更点：ライセンスを GNU AGPL v3（AGPL-3.0-or-later）に決定（§7）。
> v2.2 変更点：本家 Polis（AGPL-3.0-only）と揃えるため、AGPL-3.0-only に変更（§7）。
> v2.3 変更点：市民を本人の属性や市内の地区で分けて見せない方針を採用（§3⑤⑥、§4④。ADR 0002）。

### 1. プロジェクトの思想と目的

* **コア価値：** 各自治体の行政リソース（議事録・予算・決算）を直感的に可視化し、Polisの思想に基づいたクリーンかつ建設的な合意形成の場を全市民（特に10〜30代の若年層）に提供する。
* **展開方針：** 栃木県大田原市をパイロット自治体として立ち上げ、データ構造・クローラー・UIはすべて「自治体を差し替え可能」な形で設計する。新しい自治体の追加は「マスタデータ登録＋収集アダプタ追加」で完結し、アプリ本体の改修を不要とする。
* **開発・運用方針：** 個人開発（ワンオペレーション運用を想定し、AIによる自動化を徹底する）。本家Polisのシステムを丸ごとフォークするのではなく、Polisの「データ構造」「画面設計」「数学的ロジック（主成分分析）」のDNAを、独自のRust×Next.js環境へ移植するアプローチを採用する。本プロジェクトも同じ AGPL-3.0 とするため、Polis 由来のロジックは出典を明記したうえで参照・移植できる（§7）。
* **開発環境：** WSL（Linux/Ubuntu）上で、Claude Codeによる土台の一括生成を行い、視覚的な肉付け・ディテール開発をCursorで行う。

### 2. アプリケーション構成（モノレポ構造）

```
civic-platform/          (ルートフォルダ ※名称は自治体非依存に変更を検討)
├── backend/             (Rust / axum / sqlx) - API サーバー＋収集バッチ
│   └── src/collectors/  (自治体別の収集アダプタ)
├── frontend/            (Next.js / TypeScript / Tailwind CSS) - スマホ特化型UI
├── data/                (SQLite) - 開発初期は軽量なファイルベースDB、将来はPostgreSQLへ移行
│   └── seeds/           (自治体マスタ・地区マスタの初期データ)
└── docs/
```

### 3. データベース詳細設計 (SQLite / PostgreSQL準拠)

Polisのコアである「トピック」「ステートメント」「投票」の関係に、行政区分の階層を追加する。

#### 行政区分の階層

```
prefectures（都道府県） 1 ── n municipalities（市区町村） 1 ── n districts（地区）
                                         │
                                         └── n topics ── n statements ── n votes
```

#### ① prefectures （都道府県マスタ）【新規】

* code: TEXT / Primary Key (都道府県コード 2桁。例：'09' = 栃木県)
* name: TEXT (例：栃木県)

#### ② municipalities （市区町村マスタ）【新規】

* id: TEXT / Primary Key (総務省「全国地方公共団体コード」6桁を採用し、全国で一意かつ公的に安定したIDとする)
* prefecture_code: TEXT / Foreign Key
* name: TEXT (例：大田原市)
* name_kana: TEXT
* slug: TEXT / UNIQUE (URL用。例：'otawara')
* official_url: TEXT (公式HPトップ)
* is_active: BOOLEAN (公開中の自治体のみ選択肢に表示)
* created_at: TIMESTAMP

#### ③ districts （地区マスタ）【新規】

旧町村や地域区分など、自治体内のサブエリア。地域タグの正規化に使う。

* id: TEXT / Primary Key
* municipality_id: TEXT / Foreign Key
* name: TEXT (例：黒羽、湯津上)
* name_kana: TEXT
* ※ (municipality_id, name) に UNIQUE 制約

#### ④ municipality_stats （自治体の年度別統計）【新規】

「市民1人あたり年間〇〇円」換算に使う人口などを年度別に保持する。

* municipality_id: TEXT / Foreign Key
* fiscal_year: INTEGER
* population: INTEGER
* households: INTEGER / Nullable
* source_url: TEXT
* ※ (municipality_id, fiscal_year) を複合主キーとする

#### ⑤ users （ユーザー属性情報テーブル）【修正】

画面上は完全匿名（自動アバター）。居住地属性は自治体ごとに異なるため、usersからは切り出す。

**年齢・性別・職業などの本人の属性は集めない**（市民を属性で分けて見せると分断を煽るため。必須での収集は棄却、任意での収集も当面行わない。docs/adr/0002-participant-attributes.md）。スキーマにある `age_group` / `gender` / `occupation` の列は使わず、将来削除する候補。

* id: TEXT / Primary Key (SNS認証のユニークID)
* ~~age_group / gender / occupation~~ → 集めない（ADR 0002）
* home_municipality_id: TEXT / Foreign Key / Nullable (初期表示に使う「ホーム自治体」)
* created_at: TIMESTAMP
* ~~residency_type~~ → ⑥へ移動

#### ⑥ user_municipality_relations （ユーザーと自治体の関係）【新規】

1人のユーザーが複数自治体の議論に参加でき、自治体ごとに関係性が異なる（A市の住民でありB市の通勤者、など）ことを表現する。

* user_id: TEXT / Foreign Key
* municipality_id: TEXT / Foreign Key
* residency_type: TEXT ('resident' 住民 / 'neighbor' 周辺住民 / 'commuter' 通勤・通学 / 'related' 関係人口)
* ~~district_id~~ → **集めない**（市内の地区で市民を分けると分断を煽るため。ADR 0002）。列は残るが常に NULL
* created_at: TIMESTAMP
* ※ (user_id, municipality_id) を複合主キーとする
* その自治体のトピックで初めて投票する際に1回だけ回答を求める。

#### ⑦ topics （議論の場テーブル）【修正】

* id: TEXT / Primary Key
* municipality_id: TEXT / Foreign Key 【追加】
* title: TEXT
* description: TEXT
* source_type: TEXT ('assembly' (議事録) / 'citizen' (市民提案))
* source_url: TEXT (元PDF/テキストへのリンク)
* created_at: TIMESTAMP
* ~~tags~~ → ⑨へ正規化

#### ⑧ statements （個々の意見テーブル）【変更なし】

自治体はトピック経由で決まるため、カラム追加は不要。

* id / topic_id / user_id (Nullable：AI抽出のシード意見) / body (140文字) / status ('pending', 'approved', 'rejected', 'flagged') / created_at

#### ⑨ tags ／ topic_tags （タグの正規化）【新規】

自治体をまたいだ絞り込み（例：全国の #子育て）と、自治体内の地区絞り込みを両立するため、JSON配列から正規化テーブルへ変更する。

* tags: id / kind ('field' 分野 / 'district' 地区) / name / district_id (kind='district' の場合のみ)
* topic_tags: topic_id / tag_id（複合主キー）
* 分野タグ（#子育て, #防災 等）は全自治体共通の語彙、地区タグは districts に紐づく。

#### ⑩ votes （Polisコア投票テーブル）【変更なし】

* id / user_id / statement_id / vote_value (1: 賛成 / -1: 反対 / 0: パス) / created_at
* ※ (user_id, statement_id) の UNIQUE 制約で1人1回投票を担保。

#### ⑪ collector_sources （収集元設定）【新規】

自治体ごとに議事録システム（会議録検索システムのベンダー）や決算資料の掲載形式が異なるため、収集設定をデータとして持つ。

* id: TEXT / Primary Key
* municipality_id: TEXT / Foreign Key
* kind: TEXT ('minutes' 議事録 / 'settlement_card' 決算カード / 'budget' 予算)
* adapter: TEXT (使用する収集アダプタ名。例：'generic_html', 'kaigiroku_system_x')
* base_url: TEXT
* config: TEXT (アダプタ固有設定をJSONで保持)
* is_enabled: BOOLEAN
* last_crawled_at: TIMESTAMP / Nullable

### 4. バックエンド処理・AIパイプライン仕様 (Rust / axum)

#### ① 行政データの自動収集と可視化ロジック【修正】

* 定期バッチが collector_sources の有効な設定を順に処理する。
* 収集処理は Rust の trait（例：`Collector`）として抽象化し、自治体・システムごとの差分はアダプタ実装に閉じ込める。共通形式（会議録検索システム等）は1アダプタで複数自治体に対応する。
* 決算カードは総務省標準フォーマットのため、全自治体共通のパーサーで処理できる見込み。
* 1人あたり換算は municipality_stats の該当年度人口を用いる。
* 新規自治体の収集前に、robots.txt と各サイトの利用規約を確認する。

#### ② AI自動クレンジング（モデレーション）【変更なし】

投稿直後は 'pending' → モデレーションAPI → 'approved' / 'rejected'（マイルドな代替表現案を提示）/ 'flagged'（管理者通知）。
* 判定できないとき（通信の失敗・費用の上限）は 'pending' のまま残して再判定し、自動承認しない。モデル・基準・費用の上限は docs/adr/0003-llm-operations.md。

#### ③ AI自動タグ付け【修正】

* 分野タグは全自治体共通の語彙リストから選ばせる（表記揺れ防止）。
* 地区タグは「そのトピックの自治体の districts 一覧」をプロンプトに渡し、その中からのみ選ばせる。
* 首長・議員などの人物はタグにしない。議員ごとの記録は、会議録の発言者の記載から機械的に対応付けて扱う（docs/design/collection.md）。

#### ④ Polis分析（PCA・クラスタリング・コンセンサス判定）

* 分析単位はトピック。自治体をまたいだ集計は行わない。
* 関わり方（user_municipality_relations の residency_type）は、意見グループを作るのには使わない。できたグループの説明（どのグループにどんな関わり方の人がいるか）と、意見ごとの関わり方による違いの可視化にだけ使う（Polis の原則）。
* 本人の属性（年齢・性別など）や市内の地区では、集計も可視化もしない（ADR 0002）。
* 公表は5人未満の区分を隠し、引き算で逆算されないよう二次秘匿を行う。

### 5. フロントエンドUI/UX仕様 (Next.js / Tailwind CSS)

* **スマホ特化（モバイルファースト）**
* **URL設計：** `/{slug}`（例：`/otawara`）を自治体のトップとし、`/{slug}/topics/{id}` を詳細とする。共有されたURLだけで自治体が確定する。
* **自治体の選択：**
  * 初回アクセス時は「都道府県 → 市区町村」の2段階で選択（is_active の自治体のみ表示）。
  * 選択結果はホーム自治体として保存し、次回以降は直接そのタイムラインを開く。ヘッダーからいつでも切り替え可能。
  * 対象自治体が1つの段階では選択画面を出さず、自動でパイロット自治体を開く。
* **タイムライン画面：** 選択中の自治体のトピックを表示。分野タグ・地区タグのタップで瞬間フィルター。
* **Polis風投票画面：** 変更なし（カード1件ずつ表示、賛成／反対／パスの3択、コメント欄・リプライなし）。
* **コンセンサス・ハイライト：** 全グループで賛成率80%以上のステートメントを、その自治体のタイムライン最上位にピン留め。

### 6. セキュリティ・水増し対策と拡張ロードマップ

* **初期フェーズ（MVP）：** Google、Apple、LINEなどのSNS認証を必須とし、1人複数アカウントを抑止。
* **拡張フェーズ：** 電話番号（SMS）認証を導入。
* **アルゴリズムの防御力：** 水増しアカウントが群がっても、PCAによるクラスタリングで「1つの極端な集団」として処理されるため、コンセンサスが歪みにくい。
* **多自治体展開のロードマップ：**
  1. 大田原市のみで運用（選択UIは非表示、データ構造は多自治体対応済み）
  2. 栃木県内の近隣自治体を追加（同じ議事録システムを使う自治体から優先）
  3. 自治体追加の手順をドキュメント化（マスタ登録 → 地区登録 → 収集設定 → 人口データ登録）

### 7. ライセンスと透明性の方針【新規】

* **ライセンス：** GNU Affero General Public License v3（**AGPL-3.0-only**）。本家 Polis と同じ表記にそろえ、Polis の数理処理（PCA・クラスタリング等）を出典と著作権表示を保持して移植できるようにする。
* **採用理由：** 本プロジェクトは「行政の透明性」をコア価値とするため、サービス自体の透明性を最優先する。誰でもコードを検証・改良でき、第三者がサービスを改変して提供する場合も改良部分の公開が義務付けられ、成果が公共に還元される。Polis・Decidim・Consul Democracy など、シビックテックの代表的なプロジェクトと同じ方針である。
* **AGPL 第13条（ネットワーク利用者へのソース提供）への対応：**
  * 全ページのフッターに「ソースコード」リンク（公開リポジトリ）を常設する。
  * 本番環境のコードは常に公開リポジトリと一致させる。非公開の改造を本番に入れない。
  * API キー・管理者向けの値などの秘密情報はコードに含めず、環境変数で管理する。
* **Polis 由来のコード：** 取り込む場合は出典と著作権表示を保持し、`NOTICE` ファイルに一覧を記載する。
* **依存ライブラリ：** AGPLv3 と両立するライセンス（MIT / Apache-2.0 / BSD / MPL-2.0 / LGPL / GPLv3 等）のみ採用する。
* **外部からの貢献：** 当面は受け付けない、または受け付ける場合は DCO（Developer Certificate of Origin、コミットへの `Signed-off-by` 付与）を求める。将来の商用デュアルライセンスは、透明性を優先する方針から想定しない。
* **データのライセンス：** アプリのコードとは別に、収集・加工した行政データの公開条件（各自治体のオープンデータ利用規約、CC BY 等）を自治体ごとに確認し、出典を画面上に明記する。