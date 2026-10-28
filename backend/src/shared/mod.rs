// SPDX-License-Identifier: AGPL-3.0-only

//! システム内で共通に使うもの（設定・DB の接続・状態の型・ID と時刻・Claude API の呼び出し）。
//! ほかの層（interface・ingest・records・process・maintenance）から使い、ここからはほかの層を使わない

pub mod config;
pub mod db;
pub mod llm;
pub mod status;
pub mod util;
