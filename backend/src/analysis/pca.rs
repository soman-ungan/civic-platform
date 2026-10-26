// SPDX-License-Identifier: AGPL-3.0-only
//
// Ported from Polis (https://github.com/compdemocracy/polis)
//   math/src/polismath/math/pca.clj @ b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9
// Copyright (C) 2012-present, The Authors (Polis).
// Copyright (C) 2026 soman-ungan (Rust port).
// Licensed under the GNU Affero General Public License, version 3.

//! べき乗法による PCA と、スパース性を考慮した射影（Polis の `pca.clj`）。
//!
//! Polis との違い: 初期ベクトルが無いとき、Polis は乱数（`rand-starting-vec`）を使うが、
//! ここでは「すべて 1」のベクトルを使う（docs/polis-math.md の Q12）。結果が毎回同じになる。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pca {
    /// 列ごとの平均（中心）
    pub center: Vec<f64>,
    /// 主成分（長さ 1 のベクトル。通常は 2 本）
    pub comps: Vec<Vec<f64>>,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn length(v: &[f64]) -> f64 {
    dot(v, v).sqrt()
}

/// 長さ 1 に正規化する。ゼロベクトルはそのまま返す（vectorz の normalise と同じ）
fn normalise(v: &[f64]) -> Vec<f64> {
    let len = length(v);
    if len > 0.0 {
        v.iter().map(|x| x / len).collect()
    } else {
        v.to_vec()
    }
}

/// べき乗法の内側の1ステップ: Σ_row (row · v) row
fn xtxr(data: &[Vec<f64>], start: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; start.len()];
    for row in data {
        let coeff = dot(start, row);
        for (o, x) in out.iter_mut().zip(row) {
            *o += coeff * x;
        }
    }
    out
}

/// 第1固有ベクトルをべき乗法で求める。初期ベクトルが短ければ 1 で埋める（新しい意見の列のぶん）
pub fn power_iteration(data: &[Vec<f64>], iters: usize, start: Option<&[f64]>) -> Vec<f64> {
    let n_cols = data.first().map_or(0, Vec::len);
    let mut vector: Vec<f64> = start.map(<[f64]>::to_vec).unwrap_or_default();
    vector.truncate(n_cols);
    vector.resize(n_cols, 1.0);

    let mut iters = iters;
    let mut last_eigval = 0.0;
    loop {
        let product = xtxr(data, &vector);
        let eigval = length(&product);
        let normed = normalise(&product);
        if iters == 0 || eigval == last_eigval {
            return normed;
        }
        iters -= 1;
        vector = normed;
        last_eigval = eigval;
    }
}

/// `data` の各行から `xs` 方向の成分を取り除く（グラム・シュミット）
fn factor_matrix(data: &[Vec<f64>], xs: &[f64]) -> Vec<Vec<f64>> {
    let xx = dot(xs, xs);
    if xx == 0.0 {
        return data.to_vec();
    }
    data.iter()
        .map(|row| {
            let coeff = dot(xs, row) / xx;
            row.iter().zip(xs).map(|(r, x)| r - coeff * x).collect()
        })
        .collect()
}

/// 列平均を引いてから、主成分を `n_comps` 本求める（`powerit-pca`）。
/// `start_vectors` は前回の主成分（すべて 0 のものは使わない。`wrapped-pca` の扱い）。
pub fn powerit_pca(
    data: &[Vec<f64>],
    n_comps: usize,
    iters: usize,
    start_vectors: &[Vec<f64>],
) -> Pca {
    let n_rows = data.len();
    let n_cols = data.first().map_or(0, Vec::len);
    let center: Vec<f64> = (0..n_cols)
        .map(|j| data.iter().map(|row| row[j]).sum::<f64>() / n_rows as f64)
        .collect();
    let mut cntrd: Vec<Vec<f64>> = data
        .iter()
        .map(|row| row.iter().zip(&center).map(|(x, c)| x - c).collect())
        .collect();

    let n = n_comps.min(n_rows.min(n_cols)).max(1);
    let mut comps = Vec::with_capacity(n);
    for i in 0..n {
        let start = start_vectors
            .get(i)
            .filter(|v| v.iter().any(|x| *x != 0.0))
            .map(Vec::as_slice);
        let pc = power_iteration(&cntrd, iters, start);
        if i + 1 < n {
            cntrd = factor_matrix(&cntrd, &pc);
        }
        comps.push(pc);
    }
    Pca { center, comps }
}

/// 参加者1人の射影。投票した意見だけで内積を取り、√(意見数 / 投票数) 倍して外側へ押し出す。
/// 主成分が 2 本ない場合は (0, 0) になる（Polis の `zip` が短い方に揃えるため。Q16）。
pub fn sparsity_aware_project_ptpt(votes: &[Option<f64>], pca: &Pca) -> [f64; 2] {
    let n_cmnts = votes.len();
    let (Some(pc1), Some(pc2)) = (pca.comps.first(), pca.comps.get(1)) else {
        return [0.0, 0.0];
    };
    let mut n_votes = 0usize;
    let (mut p1, mut p2) = (0.0, 0.0);
    for (((x, c), a), b) in votes.iter().zip(&pca.center).zip(pc1).zip(pc2) {
        if let Some(x) = x {
            let x = x - c;
            n_votes += 1;
            p1 += x * a;
            p2 += x * b;
        }
    }
    let scale = (n_cmnts as f64 / n_votes.max(1) as f64).sqrt();
    [p1 * scale, p2 * scale]
}

pub fn sparsity_aware_project_ptpts(data: &[Vec<Option<f64>>], pca: &Pca) -> Vec<[f64; 2]> {
    data.iter()
        .map(|row| sparsity_aware_project_ptpt(row, pca))
        .collect()
}

/// 意見の射影: 「その意見だけに賛成（-1）した人」を射影した位置
pub fn pca_project_cmnts(pca: &Pca) -> Vec<[f64; 2]> {
    let n_cols = pca.center.len();
    (0..n_cols)
        .map(|i| {
            let mut votes = vec![None; n_cols];
            votes[i] = Some(-1.0);
            sparsity_aware_project_ptpt(&votes, pca)
        })
        .collect()
}

/// 意見の extremity（射影の長さ）
pub fn comment_extremity(projection: &[[f64; 2]]) -> Vec<f64> {
    projection
        .iter()
        .map(|[x, y]| (x * x + y * y).sqrt())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_iteration_finds_dominant_direction() {
        // 第1軸方向に大きく広がったデータ
        let data = vec![
            vec![3.0, 0.1],
            vec![-3.0, -0.1],
            vec![2.0, 0.0],
            vec![-2.0, 0.0],
        ];
        let pc = power_iteration(&data, 100, None);
        assert!((pc[0].abs() - 1.0).abs() < 1e-3);
        assert!(pc[1].abs() < 0.05);
        assert!((length(&pc) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn components_are_orthogonal() {
        let data = vec![
            vec![1.0, 2.0, 0.0],
            vec![-1.0, -2.0, 0.5],
            vec![2.0, 1.0, -1.0],
            vec![0.0, -1.0, 1.0],
            vec![-2.0, 0.0, -0.5],
        ];
        let pca = powerit_pca(&data, 2, 100, &[]);
        assert_eq!(pca.comps.len(), 2);
        assert!(dot(&pca.comps[0], &pca.comps[1]).abs() < 1e-6);
        // 中心は列平均
        assert!((pca.center[0] - 0.0).abs() < 1e-12);
        assert!((pca.center[1] - 0.0).abs() < 1e-12);
    }

    #[test]
    fn start_vector_is_padded_with_ones() {
        let data = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        // 長さ 1 の初期ベクトルでも、列数に合わせて 1 で埋めて計算できる
        let pc = power_iteration(&data, 0, Some(&[1.0]));
        assert_eq!(pc.len(), 3);
    }

    #[test]
    fn sparse_projection_scales_by_vote_ratio() {
        let pca = Pca {
            center: vec![0.0, 0.0, 0.0, 0.0],
            comps: vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]],
        };
        // 4 件中 1 件だけ投票 → √4 = 2 倍
        let p = sparsity_aware_project_ptpt(&[Some(-1.0), None, None, None], &pca);
        assert_eq!(p, [-2.0, 0.0]);
        // 未投票だけなら原点
        assert_eq!(sparsity_aware_project_ptpt(&[None; 4], &pca), [0.0, 0.0]);
    }

    #[test]
    fn single_component_projects_to_origin() {
        let pca = Pca {
            center: vec![0.0],
            comps: vec![vec![1.0]],
        };
        assert_eq!(sparsity_aware_project_ptpt(&[Some(1.0)], &pca), [0.0, 0.0]);
    }
}
