# 計画の索引

承認された計画は、実装の前にここへ保存し、実装で変えた点は各ファイルの末尾の「実装時の変更点」に書きます（CLAUDE.md の開発ルール）。
**計画は記録です。あとから中身を直しません。** 今の決まりは右の列の文書にあります（計画のあとで変わったことがあるため、今の姿はそちらを見る）。

- 2026-10-29 に `docs/cli.md`・`docs/data-flow.md` を `docs/operations/` に移しました。それより前の計画の中のパスは古いままです
- マイグレーションのコメントがこのディレクトリのファイルを参照しているので、ファイル名は変えません

## フェーズ
| 計画 | 中身 | 今の決まりの場所 |
|---|---|---|
| [2026-10-26-phase2-core](2026-10-26-phase2-core.md) | フェーズ2: トピック・投稿・投票・Polis 風の投票の画面 | [design/api.md](../design/api.md) |
| [2026-10-26-phase3-analysis](2026-10-26-phase3-analysis.md) | フェーズ3: PCA・クラスタリング・合意点 | [polis-math.md](../polis-math.md) |
| [2026-10-27-phase4-ai](2026-10-27-phase4-ai.md) | フェーズ4: モデレーション・自動のタグ付け、フェーズ5の収集の設計 | [ADR 0003](../adr/0003-llm-operations.md)・[ADR 0007](../adr/0007-statement-review.md)・[design/collection.md](../design/collection.md)・[design/tagging-criteria.md](../design/tagging-criteria.md) |
| [2026-10-27-phase5a-minutes](2026-10-27-phase5a-minutes.md) | フェーズ5a: 会議録の取り込みと AI のトピック案 | [ADR 0004](../adr/0004-assembly-minutes.md)・[operations/data-flow.md](../operations/data-flow.md) |
| [2026-10-27-phase5b-settlement-cards](2026-10-27-phase5b-settlement-cards.md) | フェーズ5b: 決算カードの収集と「まちのお金」 | [design/collection.md](../design/collection.md) |
| [2026-10-27-phase5c-budget](2026-10-27-phase5c-budget.md) | フェーズ5c: 当初予算の概要の取り込み | [design/collection.md](../design/collection.md)・[operations/cli.md](../operations/cli.md) |
| [2026-10-28-phase6a-passkeys](2026-10-28-phase6a-passkeys.md) | フェーズ6a: パスキーと運営者。**ログインの流れ（画面）はここが正** | [ADR 0006](../adr/0006-passkey-authentication.md)・[design/api.md](../design/api.md) |
| [2026-10-28-phase6b-participation-rules](2026-10-28-phase6b-participation-rules.md) | フェーズ6b: 投稿の条件・上限・意見の審査 | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md)・[ADR 0007](../adr/0007-statement-review.md)・`backend/src/process/progress.rs` |
| [2026-10-29-phase6c-operator-console](2026-10-29-phase6c-operator-console.md) | フェーズ6c: 管理画面とトピックの期限 | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md)・[design/api.md](../design/api.md) |
| [2026-10-29-phase6d-citizen-proposals](2026-10-29-phase6d-citizen-proposals.md) | フェーズ6d: 市民からのトピックの提案 | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md)・[design/api.md](../design/api.md)・`backend/src/process/progress.rs` |

## 方針・ADR
| 計画 | 中身 | 今の決まりの場所 |
|---|---|---|
| [2026-10-26-defer-deploy](2026-10-26-defer-deploy.md) | デプロイをリリースの直前にまとめる | [ADR 0001](../adr/0001-docker-adoption.md) |
| [2026-10-26-residency-visualization](2026-10-26-residency-visualization.md) | 関わり方の可視化と「分断を生まない」原則 | [ADR 0002](../adr/0002-participant-attributes.md) |
| [2026-10-27-adr-0005-policies](2026-10-27-adr-0005-policies.md) | トピックの寿命・投稿数・資金・提言を ADR に | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md) |
| [2026-10-28-adr-0005-revision](2026-10-28-adr-0005-revision.md) | 意見の上限・重複の知らせ・分析までの投票数（15件の条件は 2026-10-29 にやめた） | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md) |
| [2026-10-29-analysis-unlock-seeds](2026-10-29-analysis-unlock-seeds.md) | 分析と投稿を開く条件を「最初の意見すべて」に。自分の意見には賛成 | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md)・`backend/src/process/progress.rs` |

## 資料の取り込み
| 計画 | 中身 | 今の決まりの場所 |
|---|---|---|
| [2026-10-28-manual-extraction](2026-10-28-manual-extraction.md) | API を使わない共同作業での取り込み | [design/collection.md](../design/collection.md)・`.claude/commands/` |
| [2026-10-28-similar-topics](2026-10-28-similar-topics.md) | 似たトピック・意見の少ないトピックの扱い | [ADR 0005](../adr/0005-topic-lifecycle-and-funding.md)・[operations/cli.md](../operations/cli.md) |
| [2026-10-28-revenue-keys](2026-10-28-revenue-keys.md) | 予算の歳入の費目名を決算カードにそろえる | [design/collection.md](../design/collection.md) |

## 画面
| 計画 | 中身 |
|---|---|
| [2026-10-26-consensus-in-group-table](2026-10-26-consensus-in-group-table.md) | 合意点をグループごとの特徴の表にまとめる |
| [2026-10-26-group-table-highlight](2026-10-26-group-table-highlight.md) | グループごとの特徴の表の表記とハイライト |
| [2026-10-27-finances-ui](2026-10-27-finances-ui.md) | まちのお金: 説明の吹き出し・開閉・固定ヘッダー・推移・性質別の4区分 |
| [2026-10-27-tabs-and-fixed-header](2026-10-27-tabs-and-fixed-header.md) | タイムラインとまちのお金のスワイプ・固定ヘッダー |
| [2026-10-28-budget-projects-treemap](2026-10-28-budget-projects-treemap.md) | 今年の主な事業のツリーマップ |
| [2026-10-28-finances-unit](2026-10-28-finances-unit.md) | 「1人あたり｜総額」の切り替えをヘッダーに |
| [2026-10-28-finances-view](2026-10-28-finances-view.md) | まちのお金の表示の整理（決算と予算の切り替えなど） |
| [2026-10-28-initial-render](2026-10-28-initial-render.md) | トピックの画面の最初の表示を速くする |
| [2026-10-29-analysis-sticky-headers](2026-10-29-analysis-sticky-headers.md) | 「みんなの意見」の見出しと表の見出しの固定 |

## コードと文書の整理
| 計画 | 中身 |
|---|---|
| [2026-10-28-backend-layers](2026-10-28-backend-layers.md) | backend/src を段階ごとに階層化（今の構成は CLAUDE.md） |
| [2026-10-28-frontend-layout](2026-10-28-frontend-layout.md) | frontend/src を用途ごとに階層化（今の構成は CLAUDE.md） |
| [2026-10-28-cli-verbs](2026-10-28-cli-verbs.md) | CLI を `<動詞> <対象>` の形に（今の使い方は [operations/cli.md](../operations/cli.md)） |
| [2026-10-28-refactor-pre-phase6](2026-10-28-refactor-pre-phase6.md) | フェーズ6の前のコードの整理 |
| [2026-10-29-docs-restructure](2026-10-29-docs-restructure.md) | docs の構造化（この索引を作った） |
