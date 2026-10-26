# Polis biodiversity データ（一致テスト用）

`backend/tests/polis_parity.rs` で、Polis の Clojure 実装と計算結果が一致するかを確かめるための正解データです。

| ファイル | 内容 |
|---|---|
| `2025-11-11-1704-r4tykwac8thvzv35jrn53-votes.csv` | 投票（`vote` は賛成 = 1 / 反対 = -1 / パス = 0） |
| `r4tykwac8thvzv35jrn53_math_blob_cold_start.json` | 上の投票を Polis の Clojure 実装でまとめて計算した結果 |

- 取得元: https://github.com/compdemocracy/polis の `delphi/real_data/r4tykwac8thvzv35jrn53-biodiversity/`（コミット `b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9`）。内容は変更していません。
- ライセンス: **CC BY 4.0**（https://creativecommons.org/licenses/by/4.0/）

## 出典の表示（Polis openData の指定に従う）

Data was gathered using the Polis software (compdemocracy.org/polis) and is sub-licensed under CC BY 4.0 with Attribution to The Computational Democracy Project. The data and more information about how the data was collected can be found at the following link(s):

- https://compdemocracy.org/case-studies/2019-scoop-hivemind-biodiversity
- https://github.com/compdemocracy/openData
