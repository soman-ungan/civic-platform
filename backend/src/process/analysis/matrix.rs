// SPDX-License-Identifier: AGPL-3.0-only
//
// Ported from Polis (https://github.com/compdemocracy/polis)
//   math/src/polismath/math/named_matrix.clj @ b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9
// Copyright (C) 2012-present, The Authors (Polis).
// Copyright (C) 2026 soman-ungan (Rust port).
// Licensed under the GNU Affero General Public License, version 3.

//! 行名・列名つきの投票行列（Polis の `named_matrix.clj` のうち、分析で使う部分）。
//! 行 = 参加者、列 = 意見。値は Polis の符号（賛成 = -1、反対 = 1、パス = 0）、未投票は `None`。

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct NamedMatrix {
    rows: Vec<String>,
    cols: Vec<String>,
    row_index: HashMap<String, usize>,
    col_index: HashMap<String, usize>,
    data: Vec<Vec<Option<f64>>>,
}

impl NamedMatrix {
    /// 行と列の並びを指定して空の行列を作る（前回の並びを引き継ぐため）
    pub fn with_order(rows: Vec<String>, cols: Vec<String>) -> Self {
        let mut m = Self::default();
        for r in rows {
            m.ensure_row(&r);
        }
        for c in cols {
            m.ensure_col(&c);
        }
        m
    }

    fn ensure_row(&mut self, name: &str) -> usize {
        if let Some(&i) = self.row_index.get(name) {
            return i;
        }
        let i = self.rows.len();
        self.rows.push(name.to_owned());
        self.row_index.insert(name.to_owned(), i);
        self.data.push(vec![None; self.cols.len()]);
        i
    }

    fn ensure_col(&mut self, name: &str) -> usize {
        if let Some(&i) = self.col_index.get(name) {
            return i;
        }
        let i = self.cols.len();
        self.cols.push(name.to_owned());
        self.col_index.insert(name.to_owned(), i);
        for row in &mut self.data {
            row.push(None);
        }
        i
    }

    /// (行名, 列名, 値) を順に書き込む。新しい行・列は、出てきた順に末尾へ追加する（`update-nmat`）
    pub fn update<'a>(&mut self, values: impl IntoIterator<Item = (&'a str, &'a str, f64)>) {
        for (row, col, value) in values {
            let c = self.ensure_col(col);
            let r = self.ensure_row(row);
            self.data[r][c] = Some(value);
        }
    }

    pub fn rownames(&self) -> &[String] {
        &self.rows
    }

    pub fn colnames(&self) -> &[String] {
        &self.cols
    }

    pub fn rows(&self) -> &[Vec<Option<f64>>] {
        &self.data
    }

    pub fn row_by_name(&self, name: &str) -> Option<&[Option<f64>]> {
        self.row_index.get(name).map(|&i| self.data[i].as_slice())
    }

    pub fn col_position(&self, name: &str) -> Option<usize> {
        self.col_index.get(name).copied()
    }

    /// 指定した列を 0 で埋めた行列を返す（未投票のセルも 0 になる。`zero-out-columns`）
    pub fn zero_out_columns(&self, columns: &HashSet<String>) -> Self {
        let mut m = self.clone();
        for col in columns {
            if let Some(&c) = m.col_index.get(col) {
                for row in &mut m.data {
                    row[c] = Some(0.0);
                }
            }
        }
        m
    }

    /// 行ごとの投票数（未投票以外を数える。`user-vote-counts`）
    pub fn row_vote_counts(&self) -> Vec<(String, usize)> {
        self.rows
            .iter()
            .zip(&self.data)
            .map(|(name, row)| (name.clone(), row.iter().filter(|v| v.is_some()).count()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_appends_rows_and_cols_in_first_seen_order() {
        let mut m = NamedMatrix::default();
        m.update([("p1", "c1", -1.0), ("p2", "c2", 1.0), ("p1", "c2", 0.0)]);
        assert_eq!(m.rownames(), ["p1", "p2"]);
        assert_eq!(m.colnames(), ["c1", "c2"]);
        assert_eq!(m.rows()[0], [Some(-1.0), Some(0.0)]);
        assert_eq!(m.rows()[1], [None, Some(1.0)]);
        // 上書き
        m.update([("p2", "c2", -1.0)]);
        assert_eq!(m.rows()[1], [None, Some(-1.0)]);
    }

    #[test]
    fn with_order_keeps_previous_order() {
        let mut m = NamedMatrix::with_order(vec!["p2".into(), "p1".into()], vec!["c9".into()]);
        m.update([("p1", "c1", 1.0), ("p3", "c9", -1.0)]);
        assert_eq!(m.rownames(), ["p2", "p1", "p3"]);
        assert_eq!(m.colnames(), ["c9", "c1"]);
    }

    #[test]
    fn zero_out_columns_fills_whole_column() {
        let mut m = NamedMatrix::default();
        m.update([("p1", "c1", -1.0), ("p2", "c2", 1.0)]);
        let z = m.zero_out_columns(&HashSet::from(["c1".to_owned()]));
        assert_eq!(z.rows()[0], [Some(0.0), None]);
        assert_eq!(z.rows()[1], [Some(0.0), Some(1.0)]);
        assert_eq!(
            m.row_vote_counts(),
            [("p1".to_owned(), 1), ("p2".to_owned(), 1)]
        );
    }
}
