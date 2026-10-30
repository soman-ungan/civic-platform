// SPDX-License-Identifier: AGPL-3.0-only

//! backend の統合テスト（1つのバイナリ。`cargo test --test it [名前]`）。
//! まとまりは `src/` の階層にそろえる。テストはインメモリ SQLite にマイグレーションを当てて動かし、
//! 自治体の取り違えを防ぐため、共通の seed（`common::seed_fixture`）には2自治体を入れる。
//! 実際のバイナリを動かすテストは `cli.rs`、Polis との一致テストは `polis_parity.rs`（別のバイナリ）

mod common;

/// HTTP API（`src/interface/api/`）
mod api {
    mod analysis;
    mod auth;
    mod drafts;
    mod finances;
    mod health;
    mod operator;
    mod participant;
    mod participation;
    mod proposals;
    mod topics;
}

/// 資料を DB に入れる（`src/ingest/`）
mod ingest {
    mod budget;
    mod collect_fetch;
    mod collect_settlement_card;
    mod minutes;
    mod settlement_card;
}

/// 判定・計算（`src/process/`）。参加のルールは API を通して確かめる
mod process {
    mod analysis_store;
    mod moderation;
    mod participation_rules;
    mod review;
    mod tagging;
    mod topic_deadlines;
}

/// 共通の部品（`src/shared/`）
mod shared {
    mod llm;
    mod schema;
}

/// 準備と保守（`src/maintenance/`）
mod maintenance {
    mod eval;
    mod seeds;
}
