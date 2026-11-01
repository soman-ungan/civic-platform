# docs

読む人ごとに、何がどこにあるかをまとめます。まず [overview.md](overview.md)（今の全体像）から。

## 運営する人（データの投入・トピックの公開・意見の確認）
| 文書 | 中身 |
|---|---|
| [operations/cli.md](operations/cli.md) | 運営のコマンドの使い方と、よくある流れ（自治体の追加・決算・予算・会議録） |
| [operations/data-flow.md](operations/data-flow.md) | 資料がどこから来て、どのコマンドで DB のどこに入り、画面に出るまで。1年の作業の目安・困ったとき |
| [operations/deploy.md](operations/deploy.md) | 本番の動かし方: compose の構成・値（deploy/.env）・イメージ・更新とロールバック・コンテナの中での運営のコマンド |

下書きのトピックの公開、確認待ちの意見の採否、トピックの期限は、管理画面（`/operator`）でもできます。

## 開発する人
| 文書 | 中身 |
|---|---|
| [overview.md](overview.md) | 目的・画面・原則・構成・フェーズの状況・まだ決めていないこと |
| [design/schema.md](design/schema.md) | DB のテーブルと関係 |
| [design/api.md](design/api.md) | HTTP API（公開・参加・分析・お金・ログイン・運営者） |
| [design/collection.md](design/collection.md) | 行政資料の収集と取り込みの設計 |
| [design/tagging-criteria.md](design/tagging-criteria.md) | 自動のタグ付けの判断基準 |
| [polis-math.md](polis-math.md) | Polis の数理（PCA・クラスタリング・合意点）の移植と、一致テスト |
| [adr/](adr/) | 決めたこと（下の表） |
| [plans/](plans/README.md) | 実装の計画と、実装で変えた点（記録） |
| [grand-design.md](grand-design.md) | 当初の構想（v2）。今の実装とは違うところがある |

リポジトリでの作業の決まり（コミット・計画・テスト）は、ルートの `CLAUDE.md` にあります。

## 決めたこと（ADR）
| ADR | 中身 |
|---|---|
| [0001](adr/0001-docker-adoption.md) | Docker の導入はリリース準備でまとめて |
| [0002](adr/0002-participant-attributes.md) | 市民を属性や地区で分けない。分けて見せるのは関わり方だけ |
| [0003](adr/0003-llm-operations.md) | AI（Claude API）の運用。キーが無くても動く、判定できなければ承認しない |
| [0004](adr/0004-assembly-minutes.md) | 会議録: 名前を出すのは公人だけ、引用は原文どおり、AI のトピックは下書き |
| [0005](adr/0005-topic-lifecycle-and-funding.md) | トピックの期限・意見の数・分析を開く条件・お金と議論の切り離し・提言 |
| [0006](adr/0006-passkey-authentication.md) | 認証はパスキーだけ。保存するのは公開鍵・ID・日時 |
| [0007](adr/0007-statement-review.md) | 意見の審査（害・人物・投票できるか・似た意見）。立場では判断しない |
| [0008](adr/0008-elected-officials-and-elections.md) | 議員ごとの記録（議会で取り上げたトピックと引用）。順位・評価・立場の推測はしない |

## その他
| 文書 | 中身 |
|---|---|
| [outreach/](outreach/) | 自治体へのお願いの文（会議録のデータ提供など） |

## 置き場所の決まり
- 運営者向けは `operations/`、開発者向けの今の設計は `design/` と `overview.md`、決めたことは `adr/`、計画は `plans/`
- 計画と ADR は記録なので、あとから中身を直さない（ADR の改訂は末尾に追記する）。今の姿は `overview.md`・`design/` を直す
- マイグレーションのコメントが参照しているファイル（`grand-design.md`・`polis-math.md`・`design/collection.md`・`design/tagging-criteria.md`・`adr/`・`plans/`）は動かさない（main にマージしたマイグレーションは書き換えられないため）
