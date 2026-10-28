// SPDX-License-Identifier: AGPL-3.0-only

//! 取得した原本の置き場（`<PRIVATE_DIR>/fetched/<slug>/<対象>/`。git の管理外）。
//!
//! `fetch` が原本と記録（`<名前>.meta.json`）を置き、`extract` が AI の出力（`<名前>.extraction.json`）を
//! 足し、`import` が保存を終えたら全部を消す（原本を保持しない。docs/design/collection.md）。
//! 手でコピーした原本は、この置き場の外に置くので消さない。記録（meta）だけは隣に書く。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::fetch::CacheEntry;

/// 原本の隣に置く記録。`fetch` が書き、`extract` が足し、`import` が読む
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    /// 原本の URL（画面の出典のリンクに使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 原本へのリンクがあったページ（決算カードの人口の出典）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    /// 収集先（`collector_sources.id`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fiscal_year: Option<i32>,
    /// 条件付き取得の記録。保存が済んだら `fetch_cache` に入れる
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheEntry>,
    /// 会議名（会議録）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 開催日（会議録。YYYY-MM-DD）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_on: Option<String>,
    /// AI の出力の作り方（api / manual。`documents.extraction` に入る）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction: Option<String>,
}

impl Meta {
    /// 原本の隣の記録を読む。無ければ空
    pub fn read(original: &Path) -> anyhow::Result<Self> {
        let path = sidecar(original, "meta.json");
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| anyhow::anyhow!("{} を読めません: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(anyhow::anyhow!("{} を読めません: {e}", path.display())),
        }
    }

    pub fn write(&self, original: &Path) -> anyhow::Result<()> {
        std::fs::write(
            sidecar(original, "meta.json"),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }
}

/// 原本の隣のファイル（`2026.pdf` → `2026.extraction.json`）
pub fn sidecar(original: &Path, suffix: &str) -> PathBuf {
    let stem = original
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    original.with_file_name(format!("{stem}.{suffix}"))
}

/// 原本の隣に置くファイルの種類（原本の一覧からは外す）
const SIDECARS: [&str; 3] = ["meta.json", "extraction.json", "prompt.md"];

pub struct Store {
    root: PathBuf,
}

impl Store {
    /// `private_dir` は `PRIVATE_DIR`（既定 `../data/private`）
    pub fn new(private_dir: impl AsRef<Path>) -> Self {
        Self {
            root: private_dir.as_ref().join("fetched"),
        }
    }

    /// 1つの自治体・対象の置き場（`kind` は settlement / budget）
    pub fn dir(&self, slug: &str, kind: &str) -> PathBuf {
        self.root.join(slug).join(kind)
    }

    /// 原本と記録を置く。置いたパスを返す
    pub fn put(
        &self,
        slug: &str,
        kind: &str,
        name: &str,
        bytes: &[u8],
        meta: &Meta,
    ) -> anyhow::Result<PathBuf> {
        let dir = self.dir(slug, kind);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(name);
        std::fs::write(&path, bytes)?;
        meta.write(&path)?;
        Ok(path)
    }

    /// 置いてある原本（隣のファイルを除く。名前の順）
    pub fn originals(&self, slug: &str, kind: &str) -> anyhow::Result<Vec<PathBuf>> {
        let dir = self.dir(slug, kind);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut files = Vec::new();
        for entry in entries {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if path.is_file() && !SIDECARS.iter().any(|s| name.ends_with(&format!(".{s}"))) {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    /// この置き場に `fetch` が置いた原本か（手でコピーした原本は消さない）
    pub fn holds(&self, path: &Path) -> bool {
        match (path.canonicalize(), self.root.canonicalize()) {
            (Ok(path), Ok(root)) => path.starts_with(root),
            _ => false,
        }
    }

    /// 取り込みが済んだ原本を、隣のファイルと一緒に消す。置き場の外のものは消さない（false）
    pub fn discard(&self, original: &Path) -> anyhow::Result<bool> {
        if !self.holds(original) {
            return Ok(false);
        }
        for suffix in SIDECARS {
            let path = sidecar(original, suffix);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        std::fs::remove_file(original)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "civic-store-{name}-{}",
            crate::shared::util::new_id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn puts_lists_and_discards_with_sidecars() {
        let private = temp_dir("put");
        let store = Store::new(&private);
        let meta = Meta {
            url: Some("https://example.jp/R8.pdf".into()),
            fiscal_year: Some(2026),
            ..Meta::default()
        };
        let path = store
            .put("otawara", "budget", "2026.pdf", b"%PDF", &meta)
            .unwrap();
        std::fs::write(sidecar(&path, "extraction.json"), "{}").unwrap();
        assert_eq!(
            store.originals("otawara", "budget").unwrap(),
            vec![path.clone()]
        );
        assert_eq!(Meta::read(&path).unwrap(), meta);
        // 別の自治体の置き場とは混ざらない
        assert!(store.originals("nasu", "budget").unwrap().is_empty());

        assert!(store.discard(&path).unwrap());
        assert!(store.originals("otawara", "budget").unwrap().is_empty());
        assert!(!sidecar(&path, "meta.json").exists());
        assert!(!sidecar(&path, "extraction.json").exists());
        std::fs::remove_dir_all(private).unwrap();
    }

    #[test]
    fn does_not_discard_manual_copies() {
        let private = temp_dir("manual");
        let store = Store::new(&private);
        let manual = private.join("budget").join("R8.pdf");
        std::fs::create_dir_all(manual.parent().unwrap()).unwrap();
        std::fs::write(&manual, b"%PDF").unwrap();
        assert!(!store.discard(&manual).unwrap());
        assert!(manual.exists());
        std::fs::remove_dir_all(private).unwrap();
    }
}
