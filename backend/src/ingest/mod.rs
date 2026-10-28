// SPDX-License-Identifier: AGPL-3.0-only

//! 段階1: 資料を DB に入れる（CLI の fetch → extract → import。docs/data-flow.md）。
//! - `collect`: 原本の取得（robots.txt・間隔・条件付き取得）と置き場。決算カードの読み取りと保存
//! - `budget`・`minutes`: 当初予算の概要・会議録を AI で読み、検査して下書きに保存する
//! - `extraction`: AI の出力の出どころ（API か、共同作業で作った JSON か）

pub mod budget;
pub mod collect;
pub mod extraction;
pub mod minutes;
