// SPDX-License-Identifier: AGPL-3.0-only

//! civic-platform の backend。用途ごとの層に分ける（docs/plans/2026-10-28-backend-layers.md）。
//!
//! - `shared`: システム内で共通に使うもの（設定・DB・状態の型・ID と時刻・Claude API）
//! - `interface`: 外とのやりとり（HTTP API と運営のコマンド）
//! - 内部ロジック（CLI の段階と同じ分け方）
//!   - `ingest`: 段階1 資料を DB に入れる（fetch / extract / import）
//!   - `records`: 段階2 DB を見る・直す（list / show / edit / add / remove / publish / archive）
//!   - `process`: 段階3 DB の中身を判定・計算する（moderate / classify / analyze）
//!   - `maintenance`: 準備と保守（seed / eval）
//!
//! 依存の向きは `interface` → 内部ロジック → `shared`。内部ロジックは `interface` を使わない

pub mod ingest;
pub mod interface;
pub mod maintenance;
pub mod process;
pub mod records;
pub mod shared;
