// SPDX-License-Identifier: AGPL-3.0-only

//! 外とのやりとり。市民の画面が使う HTTP API（`api`）と、運営者が使うコマンド（`cli`）。
//! ここでは受け取り・返し・表示だけをし、中身は ingest・records・process・maintenance に任せる

pub mod api;
pub mod cli;
