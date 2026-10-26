# ADR 0001: Docker の導入範囲とタイミング

- 状態: 承認（2026-10-26 改訂: 導入のタイミングを変更。末尾の「改訂」を参照）
- 日付: 2026-10-26

## 背景

フェーズ1（土台）が完了し、デプロイ先は未定。前提は次のとおり。

- 個人開発・ワンオペ運用で、低コストと保守のしやすさを優先する
- DB は開発中 SQLite、将来 PostgreSQL へ移行する（SQL は両対応で書く）
- ライセンスは AGPL-3.0-or-later。第13条により、本番のコードは公開リポジトリと一致させる
- 開発環境は WSL（Docker / Compose は導入済み）

## 決定

**Docker は本番（ステージングを含む）の実行環境と CI に導入する。開発環境はコンテナ化しない。**

導入は次の3段階で行う。

| 時期 | 内容 |
|---|---|
| フェーズ2の開始時 | CI（GitHub Actions）の PostgreSQL サービスコンテナに、マイグレーションと seed を流す互換チェックを追加する |
| 最初のデプロイの前（フェーズ2の完了後にステージング） | backend / frontend の Dockerfile、本番用 compose、リバースプロキシ（Caddy）、SQLite のバックアップ、CI でのイメージ作成 |
| フェーズ5 / PostgreSQL 移行時 | 同じ backend イメージで収集バッチを定期実行する。PostgreSQL 移行時は compose に postgres を追加する |

デプロイ先の第一候補は「小規模な VPS 1台 + docker compose（backend / frontend / Caddy）」とする。イメージは CI でビルドし、VPS では pull するだけにする。Docker イメージとボリュームを扱える PaaS（Fly.io など）も、同じ Dockerfile で代替できる。

## 理由

1. **AGPL 第13条の担保**: Dockerfile と compose をリポジトリに置くことで、本番の組み立て方を第三者が再現できる。イメージにコミット SHA を埋め込み画面に表示すれば、本番がどのコミットかを利用者が確認できる。
2. **ワンオペでの再現性とロールバック**: 動作していたタグのイメージに戻すだけで切り戻せる。サーバーに Rust / Node のツールチェーンを入れなくてよい。
3. **SQLite の永続化が単純**: ボリュームを1つ置くだけでよい。マイグレーションは `sqlx::migrate!()` でバイナリに埋め込まれているため、イメージ単体で起動時に適用される。
4. **PostgreSQL 互換の機械的な検証**: 現在の SQL（BOOLEAN、部分インデックス、複合外部キー、`ON CONFLICT DO NOTHING`）は PostgreSQL 互換のはずだが未検証。API のクエリが増える前に CI で確認できるようにする。

開発環境をコンテナ化しない理由:

- Rust のインクリメンタルビルドや Cursor / rust-analyzer との連携はネイティブのほうが速く扱いやすい。WSL 上でもバインドマウント経由のビルドは遅い。
- 現状の `cargo run` / `npm run dev` で困っていない。DB はファイル1つで済む。

## 却下した案

- **サーバーレス（Cloud Run 型）**: SQLite（WAL）はローカルの永続ディスクと単一の書き込みプロセスが前提で、使い捨てのインスタンスや複数台構成と合わない。
- **frontend だけ Vercel に置く構成**: 運用先が2つに分かれ、CORS の考慮も必要になるため、ワンオペには不利。
- **開発環境も含めた全面的なコンテナ化**: 上記の理由により採用しない。

## 導入時の注意点

- **`NEXT_PUBLIC_*` はビルド時に埋め込まれる**: `NEXT_PUBLIC_SOURCE_URL` などは build-arg で渡すため、frontend のイメージは環境ごとに作ることになる。API を Caddy で同一オリジンの `/api` に振り分ければ、API の URL の埋め込みと CORS が不要になる（フェーズ2で API を作るときに決める）。
- **`frontend/next.config.ts`**: ルートの `.env` の読み込みは `existsSync` で囲ってあるので、コンテナ内に `.env` がなくても動く。`NEXT_PUBLIC_SOURCE_URL` 必須のチェックは `next start` では起動時にも実行される。`output: "standalone"` の挙動は、実装時に `node_modules/next/dist/docs/` で確認する。
- **backend のイメージ**: マルチステージビルドにする。`data/seeds/` をイメージに含めて `SEEDS_DIR` で指定し、`DATABASE_URL` はボリューム上の絶対パスにする。seed の投入は `backend seed` サブコマンドで行う。
- **バックアップ**: SQLite を外部ストレージへ継続的に複製する（例: Litestream）。最低でも定期的に `sqlite3 .backup` を取る。
- **収集バッチ（フェーズ5）**: backend イメージのサブコマンドとして定期実行する。HTTP クライアントは rustls 系にして、OpenSSL への依存を避ける。
- **ライセンス**: ベースイメージや Caddy / Litestream などが AGPLv3 と両立するかを、採用時に確認する。
- **Public 化**: 本番デプロイの前に GitHub リポジトリを Public に切り替える（CLAUDE.md「ライセンス」節）。

## 改訂（2026-10-26）: デプロイ基盤はリリース準備のときにまとめて作る

**変更内容**: 「最初のデプロイの前（フェーズ2の完了後にステージング）」としていた段階を取りやめる。デプロイ先の選定・契約、Dockerfile・本番用 compose・Caddy・バックアップ・ドメインの整備は、全体の機能がそろった後の**リリース準備（CLAUDE.md の開発フェーズ7）**でまとめて行う。

**理由**: 全体の機能がそろってから、デプロイ先を選んで契約したいというユーザーの判断。

**変えないこと**
- Docker は本番の実行環境と CI に限って導入し、開発環境はコンテナ化しない
- CI の PostgreSQL 互換チェック（第1段階）は実施済みで、そのまま続ける
- 上の「導入時の注意点」は、リリース準備のときにそのまま使う

**影響**
- 本番の公開は認証（フェーズ6）の後なので、ステージングがなくても公開前の判断には影響しない
- スマホ実機での確認は、同じ LAN 内から開発サーバーにつないで行う
- 本番特有の問題（HTTPS、Cookie の Secure 属性、イメージ化、バックアップの復元）は、リリース準備のときにまとめて出てくる。準備の期間を長めに見ておく
- 収集バッチ（フェーズ5）は、まず backend のサブコマンドとして作り、手元で実行する。定期実行の仕組みはリリース準備のときに作る

### 参考: 2026-10 時点のデプロイ先の調査

**契約の前に、料金と条件を必ず改めて確認すること。** 料金は税込・公式ページの表示で、ドル建ては為替の影響を受ける。

| 候補 | 月額の目安 | 特徴 |
|---|---|---|
| さくらのVPS 2GB | 石狩 1,738円 / 東京 1,958円（月払い） | 円建て・初期費用なし。最低3か月、時間課金なし |
| ConoHa VPS 2GB | 時間課金は上限 2,033円。長期契約（まとめトク）で 677〜1,007円（更新時 903〜1,259円） | 時間課金で試しやすい。長期契約の割引は更新時に高くなる |
| Fly.io（東京、倍率 1.308） | backend 256MB $2.87 + frontend 512MB 約 $5.74 + ボリューム $0.15/GB ≒ $9 | サーバー管理は不要。ドル建て。2026-10 に値上げ。同一オリジンの `/api` 構成に工夫が要る |
| AWS Lightsail | $5〜$12 | ドル建て。転送量の超過は $0.14/GB（東京） |
| Oracle Cloud 無料枠 | 0円 | 2026年に縮小（2 OCPU / 12GB）。東京は空きがないことが多く、本番には不向き |

- バックアップ先: Cloudflare R2 の無料枠（Standard 10GB/月、転送は無料）で、SQLite の複製は収まる見込み
- 構成に必要なメモリ（backend + Next.js + Caddy + Litestream）は合計で約 0.5GB。運用と分析処理の余裕を見て 2GB を想定
- 当時の暫定の第一候補: **さくらのVPS 2GB + docker compose（Caddy / Litestream）**。ステージングはベーシック認証で本人だけが使い、イメージは CI でビルドして SSH で送る想定だった

出典:
- さくらのVPS 仕様・料金: https://vps.sakura.ad.jp/specification/
- ConoHa VPS 料金: https://vps.conoha.jp/pricing/
- Fly.io Pricing: https://fly.io/pricing/
- AWS Lightsail FAQ: https://aws.amazon.com/lightsail/faq/
- Cloudflare R2 Pricing: https://developers.cloudflare.com/r2/pricing/
- Oracle Cloud 無料枠の縮小: https://terminalbytes.com/oracle-cloud-free-tier-changes-2026/
