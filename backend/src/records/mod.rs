// SPDX-License-Identifier: AGPL-3.0-only

//! 段階2: DB を見る・直す（CLI の list / show / edit / add / remove / publish / archive）。
//! 下書きは API のどこにも出ない。運営者がここで確かめてから公開する（ADR 0004）。
//! 認証（フェーズ6）のあとの管理画面も、ここの関数を使う

pub mod accounts;
pub mod budget;
pub mod minutes;
pub mod proposals;
pub mod publishing;
pub mod recommendations;
pub mod settlement;
pub mod statements;
pub mod tags;
pub mod topics;
