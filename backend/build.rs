// SPDX-License-Identifier: AGPL-3.0-only

// sqlx::migrate!() はマイグレーションをコンパイル時に埋め込むため、
// migrations/ が変わったら再ビルドさせる（sqlx 公式の推奨設定）。
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
