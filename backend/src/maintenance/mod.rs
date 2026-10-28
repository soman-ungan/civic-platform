// SPDX-License-Identifier: AGPL-3.0-only

//! 準備と保守（CLI の seed・eval）。
//! - `seed`: リポジトリの seed（マスタ・サンプル）を DB に入れる
//! - `eval`: AI のモデレーション・タグ付けの精度を測る（実際の API を呼ぶので費用がかかる）

pub mod eval;
pub mod seed;
