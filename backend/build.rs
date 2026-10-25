// SPDX-License-Identifier: AGPL-3.0-or-later

// sqlx::migrate!() はマイグレーションをコンパイル時に埋め込むため、
// migrations/ が変わったら再ビルドさせる（sqlx 公式の推奨設定）。
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
