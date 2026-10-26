# ADR 0001: Docker の導入範囲とタイミング

- 状態: 承認
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
