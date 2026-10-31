// SPDX-License-Identifier: AGPL-3.0-only

//! 段階3: DB の中身を判定・計算する（CLI の moderate / classify / analyze）。
//! どれも API サーバーが裏で動かしている処理で、市民の投稿・投票に対して働く。
//! - `moderation`: 投稿された意見の判定（ADR 0003）
//! - `tagging`: トピックの分野・地区のタグ付け（CLI の classify）
//! - `analysis`: Polis の数理（PCA・クラスタリング・代表意見・合意点。docs/polis-math.md）
//! - `progress`: 参加のルール（進み具合・分析を見られるか・参加できるトピックか）
//! - `publishing`: 下書きの自動公開（公開待ちを、決まった時刻に1件ずつ）
//! - `topic_map`: トピックマップ（締め切ったトピックを似ている度合いで配置する）と近傍
//! - `digest`: 市民のまとめ（選んだトピックの合意点・意見が分かれた点・代表的な意見。ADR 0005）
//! - `similarity`: 文の似ている度合い（トピックと意見の類似判定の候補を絞る）

pub mod analysis;
pub mod digest;
pub mod moderation;
pub mod progress;
pub mod publishing;
pub mod similarity;
pub mod tagging;
pub mod topic_map;
