// SPDX-License-Identifier: AGPL-3.0-only

//! 投票の分析（PCA・クラスタリング・代表意見・合意点）。
//! 数理処理は Polis の Clojure 実装からの移植（docs/polis-math.md）。

pub mod clusters;
pub mod conversation;
pub mod matrix;
pub mod pca;
pub mod repness;
pub mod stats;
pub mod store;
