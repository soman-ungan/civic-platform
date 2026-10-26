// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{SecondsFormat, Utc};

/// 新しい ID（UUID v7。時刻順に並ぶ）
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// 現在時刻の ISO8601 文字列（UTC、ミリ秒まで）。例: `2026-10-26T12:34:56.789Z`
pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
