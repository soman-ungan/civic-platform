// SPDX-License-Identifier: AGPL-3.0-only

//! AI に資料を読ませる作業（会議録からトピック案、予算の概要の書き写し）の、出力の出どころ。
//! API で呼ぶか、運営者が Claude Code などとの共同作業で作った JSON を渡す（manual）。
//! CLI では `extract` が API を呼んで JSON に書き、`import` がその JSON を読む（`Saved`）。
//! どれでも、出力は同じ検査（`minutes::extract::check`・`budget::check`）を通る
//! （docs/plans/2026-10-28-manual-extraction.md）

use std::path::Path;

use serde_json::Value;

use crate::shared::llm::Llm;

/// AI の出力をどこから得るか
pub enum Extraction<'a> {
    /// Claude API を呼ぶ（費用がかかる。`llm_usage` に記録される）
    Api { llm: &'a Llm, model: &'a str },
    /// 共同作業で作った出力（`Prompt` の形に従う JSON）
    Manual(Value),
    /// `extract` が API で作って保存した出力（作り方は api として残す）
    Saved(Value),
}

impl Extraction<'_> {
    /// `documents.extraction` に残す値
    pub fn method(&self) -> &'static str {
        match self {
            Self::Api { .. } | Self::Saved(_) => "api",
            Self::Manual(_) => "manual",
        }
    }

    /// 保存してある出力（JSON のファイル）を読む。`method` は作り方（api / manual。原本の記録の `extraction`）
    pub fn from_file(path: &Path, method: &str) -> anyhow::Result<Extraction<'static>> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("{} を読めません: {e}", path.display()))?;
        let value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("{} が JSON ではありません: {e}", path.display()))?;
        Ok(match method {
            "api" => Extraction::Saved(value),
            _ => Extraction::Manual(value),
        })
    }

    /// 出力の値（API を呼ぶものは無い）
    pub fn saved_value(&self) -> Option<&Value> {
        match self {
            Self::Api { .. } => None,
            Self::Manual(value) | Self::Saved(value) => Some(value),
        }
    }
}

/// AI に渡すもの（API に送るものと同じ）。共同作業では、これを書き出して読んでもらう
pub struct Prompt {
    pub system: String,
    pub user: String,
    /// 出力の形（JSON Schema）
    pub schema: Value,
    /// 入力に添える資料（予算の概要の PDF のパスなど）。API では本文に添付する
    pub attachment: Option<String>,
}

impl Prompt {
    /// 共同作業のための1つの文書（Markdown）にする
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str("# 指示\n\n");
        s.push_str(&self.system);
        s.push_str("\n\n# 出力の形\n\n");
        s.push_str(
            "次の JSON Schema に従う JSON だけを、ファイルに書く（説明の文は付けない）。\n\n```json\n",
        );
        s.push_str(&serde_json::to_string_pretty(&self.schema).unwrap_or_default());
        s.push_str("\n```\n\n# 入力\n\n");
        if let Some(path) = &self.attachment {
            s.push_str(&format!("添付の資料: `{path}`（このファイルを読む）\n\n"));
        }
        s.push_str(&self.user);
        s.push('\n');
        s
    }
}
