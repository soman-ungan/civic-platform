// SPDX-License-Identifier: AGPL-3.0-only
//
// Ported from Polis (https://github.com/compdemocracy/polis)
//   math/src/polismath/math/clusters.clj @ b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9
// Copyright (C) 2012-present, The Authors (Polis).
// Copyright (C) 2026 soman-ungan (Rust port).
// Licensed under the GNU Affero General Public License, version 3.

//! 重み付き k-means とシルエット係数（Polis の `clusters.clj`）。
//!
//! 同点の扱いは Clojure の `min-key` / `max-key` と同じく「後に出てきたものが勝つ」。
//! 距離は通常のユークリッド距離で計算する（vectorz の桁落ち Q11 や、ハッシュ順による同点の
//! 解決 Q17 は再現しない。docs/polis-math.md）。

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cluster<K> {
    pub id: i64,
    pub members: Vec<K>,
    pub center: Vec<f64>,
}

/// 行名つきの点の集まり（行の順序に意味がある）
pub struct Points<'a, K> {
    pub ids: &'a [K],
    pub coords: &'a [Vec<f64>],
}

impl<K: Clone + Eq + Hash> Points<'_, K> {
    fn index(&self) -> HashMap<&K, usize> {
        self.ids.iter().enumerate().map(|(i, k)| (k, i)).collect()
    }
}

pub fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// `xs` の中で `f` が最小のもの。同点なら後のもの（Clojure の `min-key`）
fn min_by_last<T>(xs: impl IntoIterator<Item = T>, f: impl Fn(&T) -> f64) -> Option<T> {
    let mut best: Option<(f64, T)> = None;
    for x in xs {
        let v = f(&x);
        if best.as_ref().is_none_or(|(b, _)| v <= *b) {
            best = Some((v, x));
        }
    }
    best.map(|(_, x)| x)
}

/// `xs` の中で `f` が最大のもの。同点なら後のもの（Clojure の `max-key`）
fn max_by_last<T>(xs: impl IntoIterator<Item = T>, f: impl Fn(&T) -> f64) -> Option<T> {
    let mut best: Option<(f64, T)> = None;
    for x in xs {
        let v = f(&x);
        if best.as_ref().is_none_or(|(b, _)| v >= *b) {
            best = Some((v, x));
        }
    }
    best.map(|(_, x)| x)
}

/// 平均（重みがあれば重み付き平均）。`weighted-mean` の行列版
pub fn weighted_mean(rows: &[&[f64]], weights: Option<&[f64]>) -> Vec<f64> {
    let dim = rows.first().map_or(0, |r| r.len());
    let mut out = vec![0.0; dim];
    match weights {
        Some(w) => {
            let total: f64 = w.iter().sum();
            for (row, wi) in rows.iter().zip(w) {
                for (o, x) in out.iter_mut().zip(row.iter()) {
                    *o += x * wi;
                }
            }
            out.iter_mut().for_each(|o| *o /= total);
        }
        None => {
            for row in rows {
                for (o, x) in out.iter_mut().zip(row.iter()) {
                    *o += x;
                }
            }
            out.iter_mut().for_each(|o| *o /= rows.len() as f64);
        }
    }
    out
}

/// メンバーの座標から中心を求める（`weights` は メンバー → 重み）
fn members_center<K: Eq + Hash>(
    points: &Points<'_, K>,
    index: &HashMap<&K, usize>,
    members: &[K],
    weights: Option<&HashMap<K, f64>>,
) -> Option<Vec<f64>> {
    // 行の順序で並べる（rowname-subset は元の順序を保つ）
    let mut present: Vec<usize> = members
        .iter()
        .filter_map(|m| index.get(m).copied())
        .collect();
    if present.is_empty() {
        return None;
    }
    present.sort_unstable();
    present.dedup();
    let rows: Vec<&[f64]> = present
        .iter()
        .map(|&i| points.coords[i].as_slice())
        .collect();
    let w: Option<Vec<f64>> = weights.map(|w| {
        present
            .iter()
            .map(|&i| w.get(&points.ids[i]).copied().unwrap_or(1.0))
            .collect()
    });
    Some(weighted_mean(&rows, w.as_deref()))
}

/// 異なる点を先頭から k 個選んで、初期クラスタにする（`init-clusters`）
pub fn init_clusters<K>(points: &Points<'_, K>, k: usize) -> Vec<Cluster<K>> {
    let mut seen: Vec<&Vec<f64>> = Vec::new();
    for c in points.coords {
        if !seen.contains(&c) {
            seen.push(c);
        }
        if seen.len() == k {
            break;
        }
    }
    seen.into_iter()
        .enumerate()
        .map(|(i, c)| Cluster {
            id: i as i64,
            members: Vec::new(),
            center: c.clone(),
        })
        .collect()
}

/// k-means の1ステップ: 各点を最も近い中心に割り当て、中心を更新する。空のクラスタは除く
fn cluster_step<K: Clone + Eq + Hash>(
    points: &Points<'_, K>,
    clusters: &[Cluster<K>],
    weights: Option<&HashMap<K, f64>>,
) -> Vec<Cluster<K>> {
    let mut assigned: Vec<Vec<usize>> = vec![Vec::new(); clusters.len()];
    for (i, coord) in points.coords.iter().enumerate() {
        let best = min_by_last(0..clusters.len(), |&c| distance(coord, &clusters[c].center))
            .expect("クラスタが空");
        assigned[best].push(i);
    }
    clusters
        .iter()
        .zip(assigned)
        .filter(|(_, idx)| !idx.is_empty())
        .map(|(c, idx)| {
            let rows: Vec<&[f64]> = idx.iter().map(|&i| points.coords[i].as_slice()).collect();
            let w: Option<Vec<f64>> = weights.map(|w| {
                idx.iter()
                    .map(|&i| w.get(&points.ids[i]).copied().unwrap_or(1.0))
                    .collect()
            });
            Cluster {
                id: c.id,
                members: idx.iter().map(|&i| points.ids[i].clone()).collect(),
                center: weighted_mean(&rows, w.as_deref()),
            }
        })
        .collect()
}

/// 2つのクラスタリングの中心が、どれも 0.01 未満の距離に収まっているか（`same-clustering?`）
fn same_clustering<K>(a: &[Cluster<K>], b: &[Cluster<K>]) -> bool {
    let sorted = |cs: &[Cluster<K>]| {
        let mut centers: Vec<Vec<f64>> = cs.iter().map(|c| c.center.clone()).collect();
        centers.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        centers
    };
    // Clojure の zip と同じく、短い方に合わせる
    sorted(a)
        .iter()
        .zip(sorted(b).iter())
        .all(|(x, y)| distance(x, y) < 0.01)
}

fn merge_clusters<K: Clone>(a: &Cluster<K>, b: &Cluster<K>) -> Cluster<K> {
    // 人数の多い方の id を引き継ぐ（同数なら後者。max-key）
    let id = if b.members.len() >= a.members.len() {
        b.id
    } else {
        a.id
    };
    let mut members = a.members.clone();
    members.extend(b.members.iter().cloned());
    let center = weighted_mean(
        &[a.center.as_slice(), b.center.as_slice()],
        Some(&[a.members.len() as f64, b.members.len() as f64]),
    );
    Cluster {
        id,
        members,
        center,
    }
}

/// 中心がまったく同じクラスタを1つにまとめる（`uniqify-clusters`）
fn uniqify_clusters<K: Clone>(clusters: Vec<Cluster<K>>) -> Vec<Cluster<K>> {
    let mut out: Vec<Cluster<K>> = Vec::new();
    for c in clusters {
        if let Some(pos) = out.iter().position(|o| o.center == c.center) {
            out[pos] = merge_clusters(&out[pos], &c);
        } else {
            out.push(c);
        }
    }
    out
}

/// 最寄りの中心から最も遠い点（`most-distal`）。戻り値は (距離, 点の位置)
fn most_distal<K>(points: &Points<'_, K>, clusters: &[Cluster<K>]) -> Option<(f64, usize)> {
    let per_point = points.coords.iter().enumerate().map(|(i, coord)| {
        let nearest =
            min_by_last(clusters.iter().map(|c| distance(coord, &c.center)), |d| *d).unwrap_or(0.0);
        (nearest, i)
    });
    max_by_last(per_point, |(d, _)| *d)
}

/// 前回のクラスタを、今回のデータに合わせて整える（`clean-start-clusters`）。
/// 前回のクラスタが無ければ `init-clusters` を使う。
fn clean_start_clusters<K: Clone + Eq + Hash>(
    points: &Points<'_, K>,
    last: &[Cluster<K>],
    k: usize,
    weights: Option<&HashMap<K, f64>>,
) -> Vec<Cluster<K>> {
    if last.is_empty() {
        return init_clusters(points, k);
    }
    let index = points.index();

    // safe-recenter: 今回のデータにメンバーが残っていないクラスタは除く
    let mut clusters: Vec<Cluster<K>> = last
        .iter()
        .filter_map(|c| {
            members_center(points, &index, &c.members, weights).map(|center| Cluster {
                id: c.id,
                members: c.members.clone(),
                center,
            })
        })
        .collect();
    if clusters.is_empty() {
        let all: Vec<K> = points.ids.to_vec();
        let center = members_center(points, &index, &all, weights).unwrap_or_default();
        clusters = vec![Cluster {
            id: last.iter().map(|c| c.id).max().unwrap_or(-1) + 1,
            members: all,
            center,
        }];
    }
    let mut clusters = uniqify_clusters(clusters);

    let mut distinct: Vec<&Vec<f64>> = Vec::new();
    for c in points.coords {
        if !distinct.contains(&c) {
            distinct.push(c);
        }
    }
    let possible = k.min(distinct.len());

    loop {
        // recenter
        for c in &mut clusters {
            if let Some(center) = members_center(points, &index, &c.members, weights) {
                c.center = center;
            }
        }
        if possible <= clusters.len() {
            return clusters;
        }
        let Some((dist, i)) = most_distal(points, &clusters) else {
            return clusters;
        };
        if dist <= 0.0 {
            return clusters;
        }
        // 最も遠い点を元のクラスタから外し、その点だけの新しいクラスタを作る
        let outlier = points.ids[i].clone();
        for c in &mut clusters {
            c.members.retain(|m| *m != outlier);
        }
        let new_id = clusters.iter().map(|c| c.id).max().unwrap_or(-1) + 1;
        clusters.push(Cluster {
            id: new_id,
            members: vec![outlier],
            center: points.coords[i].clone(),
        });
    }
}

/// 重み付き k-means（`kmeans`）。`max_iters` の既定は Polis と同じく 20
pub fn kmeans<K: Clone + Eq + Hash>(
    points: &Points<'_, K>,
    k: usize,
    last: Option<&[Cluster<K>]>,
    max_iters: usize,
    weights: Option<&HashMap<K, f64>>,
) -> Vec<Cluster<K>> {
    let mut clusters = match last {
        Some(last) => clean_start_clusters(points, last, k, weights),
        None => init_clusters(points, k),
    };
    let mut iter = max_iters;
    loop {
        let new = cluster_step(points, &clusters, weights);
        if iter == 0 || same_clustering(&clusters, &new) {
            let mut new = new;
            new.sort_by_key(|c| c.id);
            return new;
        }
        clusters = new;
        iter -= 1;
    }
}

/// クラスタリング全体のシルエット係数（各メンバーの値の単純平均）。
/// `dist` はメンバー同士の距離。1人だけのクラスタのメンバーは 0。
pub fn silhouette<K: Clone + Eq + Hash>(
    order: &[K],
    dist: &HashMap<(K, K), f64>,
    clusters: &[Cluster<K>],
) -> f64 {
    let in_clusters: HashSet<&K> = clusters.iter().flat_map(|c| c.members.iter()).collect();
    let members: Vec<&K> = order.iter().filter(|m| in_clusters.contains(m)).collect();
    if members.is_empty() {
        return 0.0;
    }
    let total: f64 = members
        .iter()
        .map(|&member| {
            let mut a: Option<f64> = None;
            let mut b: Option<f64> = None;
            for c in clusters {
                let mine = c.members.contains(member);
                let others: Vec<&K> = c.members.iter().filter(|m| *m != member).collect();
                if mine && others.is_empty() {
                    return 0.0;
                }
                if others.is_empty() {
                    continue;
                }
                let mean = others
                    .iter()
                    .map(|o| dist[&((*member).clone(), (*o).clone())])
                    .sum::<f64>()
                    / others.len() as f64;
                if mine {
                    a = Some(mean);
                } else {
                    b = Some(b.map_or(mean, |b| b.min(mean)));
                }
            }
            match (a, b) {
                // 他のクラスタが無い（全員が1つのクラスタ）ときは 0 とする
                (Some(a), Some(b)) => (b - a) / b.max(a),
                _ => 0.0,
            }
        })
        .sum();
    total / members.len() as f64
}

/// 点同士の距離表
pub fn dist_matrix<K: Clone + Eq + Hash>(points: &Points<'_, K>) -> HashMap<(K, K), f64> {
    let mut out = HashMap::new();
    for (i, a) in points.ids.iter().enumerate() {
        for (j, b) in points.ids.iter().enumerate() {
            out.insert(
                (a.clone(), b.clone()),
                distance(&points.coords[i], &points.coords[j]),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(coords: &[[f64; 2]]) -> (Vec<String>, Vec<Vec<f64>>) {
        let ids = (0..coords.len()).map(|i| format!("p{i}")).collect();
        let coords = coords.iter().map(|c| c.to_vec()).collect();
        (ids, coords)
    }

    #[test]
    fn init_clusters_takes_first_k_distinct_points() {
        let (ids, coords) = pts(&[[0.0, 0.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]);
        let p = Points {
            ids: &ids,
            coords: &coords,
        };
        let init = init_clusters(&p, 2);
        assert_eq!(init.len(), 2);
        assert_eq!(init[0].center, [0.0, 0.0]);
        assert_eq!(init[1].center, [1.0, 1.0]);
    }

    #[test]
    fn kmeans_separates_two_blobs() {
        let (ids, coords) = pts(&[
            [0.0, 0.0],
            [10.0, 10.0],
            [0.2, 0.1],
            [10.1, 9.9],
            [0.1, 0.2],
            [9.8, 10.2],
        ]);
        let p = Points {
            ids: &ids,
            coords: &coords,
        };
        let clusters = kmeans(&p, 2, None, 20, None);
        assert_eq!(clusters.len(), 2);
        let sets: Vec<HashSet<&String>> = clusters
            .iter()
            .map(|c| c.members.iter().collect())
            .collect();
        assert!(
            sets.iter()
                .any(|s| s.len() == 3 && s.contains(&ids[0]) && s.contains(&ids[2]))
        );
        let sil = silhouette(&ids, &dist_matrix(&p), &clusters);
        assert!(
            sil > 0.9,
            "はっきり分かれた2つの塊のシルエットは 1 に近い: {sil}"
        );
    }

    #[test]
    fn weighted_mean_uses_weights() {
        let m = weighted_mean(&[&[0.0, 0.0], &[4.0, 8.0]], Some(&[3.0, 1.0]));
        assert_eq!(m, [1.0, 2.0]);
    }

    #[test]
    fn clean_start_splits_until_k_clusters() {
        let (ids, coords) = pts(&[[0.0, 0.0], [0.1, 0.0], [5.0, 5.0], [9.0, 0.0]]);
        let p = Points {
            ids: &ids,
            coords: &coords,
        };
        // 前回は全員が1つのクラスタ
        let last = vec![Cluster {
            id: 0,
            members: ids.clone(),
            center: vec![0.0, 0.0],
        }];
        let clusters = kmeans(&p, 3, Some(&last), 20, None);
        assert_eq!(clusters.len(), 3);
        // 前回の id 0 は引き継がれる
        assert!(clusters.iter().any(|c| c.id == 0));
    }

    #[test]
    fn singleton_member_has_zero_silhouette() {
        let (ids, coords) = pts(&[[0.0, 0.0], [10.0, 0.0], [10.5, 0.0]]);
        let p = Points {
            ids: &ids,
            coords: &coords,
        };
        let clusters = vec![
            Cluster {
                id: 0,
                members: vec![ids[0].clone()],
                center: vec![0.0, 0.0],
            },
            Cluster {
                id: 1,
                members: vec![ids[1].clone(), ids[2].clone()],
                center: vec![10.25, 0.0],
            },
        ];
        let d = dist_matrix(&p);
        // p0 は 1人だけのクラスタなので 0。p1・p2 は (b - a) / max(b, a)
        // （a = 同じクラスタの他メンバーまでの平均距離、b = 他クラスタのメンバーまでの平均距離）
        let s1 = (10.0 - 0.5) / 10.0;
        let s2 = (10.5 - 0.5) / 10.5;
        let expected = (0.0 + s1 + s2) / 3.0;
        assert!((silhouette(&ids, &d, &clusters) - expected).abs() < 1e-9);
    }

    #[test]
    fn ties_go_to_the_last_candidate() {
        assert_eq!(min_by_last([3, 1, 1], |x| *x as f64), Some(1));
        assert_eq!(
            min_by_last([(1.0, "a"), (1.0, "b")], |x| x.0).unwrap().1,
            "b"
        );
        assert_eq!(
            max_by_last([(2.0, "a"), (2.0, "b")], |x| x.0).unwrap().1,
            "b"
        );
    }
}
