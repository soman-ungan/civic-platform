// SPDX-License-Identifier: AGPL-3.0-only
//
// Ported from Polis (https://github.com/compdemocracy/polis)
//   math/src/polismath/math/conversation.clj @ b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9
//   （small-conv-update-graph と、その中で使う関数）
// Copyright (C) 2012-present, The Authors (Polis).
// Copyright (C) 2026 soman-ungan (Rust port).
// Licensed under the GNU Affero General Public License, version 3.

//! 1つの会話（このプロジェクトではトピック）の分析を、前回の状態から更新する。
//! Polis との違いは docs/polis-math.md を参照（サブグループは計算しない、優先度は今回の
//! group-votes から出す、データが少なすぎるときは計算しない、など）。

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::clusters::{Cluster, Points, dist_matrix, kmeans, silhouette};
use super::matrix::NamedMatrix;
use super::pca::{
    Pca, comment_extremity, pca_project_cmnts, powerit_pca, sparsity_aware_project_ptpt,
    sparsity_aware_project_ptpts,
};
use super::repness::{
    Consensus, RepComment, conv_repness, select_consensus_comments, select_rep_comments,
};

/// Polis の既定値（`base-conv-update-graph` の `:opts'`）
pub const N_COMPS: usize = 2;
pub const PCA_ITERS: usize = 100;
pub const BASE_K: usize = 100;
pub const BASE_ITERS: usize = 100;
pub const MAX_K: usize = 5;
/// グループの k-means の反復回数。Polis は `:group-iters 100` を渡しているが、
/// キー名の違いで無視され、実際には既定の 20 で動いている（Q3）。動いている値に合わせる
pub const GROUP_ITERS: usize = 20;
pub const GROUP_K_BUFFER: u32 = 4;
/// in-conv: この件数（意見が少なければ全件）以上投票した人を分析に含める
pub const IN_CONV_MIN_VOTES: usize = 7;
/// in-conv が少ないときに、投票数の多い順に補う人数
pub const GREEDY_N: usize = 15;
/// 表示順の優先度で、メタ意見に使う値（このプロジェクトにメタ意見はない）
pub const META_PRIORITY: f64 = 7.0;

/// 1票。`value` は Polis の符号（賛成 = -1、反対 = 1、パス = 0）
#[derive(Debug, Clone)]
pub struct Vote {
    pub pid: String,
    pub tid: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KSmoother {
    pub last_k: usize,
    pub last_k_count: u32,
    pub smoothed_k: usize,
}

/// 次回の計算に引き継ぐ状態（Polis の conv の一部）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvState {
    /// 行（参加者）と列（意見）の並び。最初に投票された順
    pub row_order: Vec<String>,
    pub col_order: Vec<String>,
    /// 一度入った参加者は外れない
    pub in_conv: Vec<String>,
    pub pca: Option<Pca>,
    pub base_clusters: Vec<Cluster<String>>,
    pub group_clusterings: BTreeMap<usize, Vec<Cluster<i64>>>,
    pub group_k_smoother: Option<KSmoother>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct VoteCounts {
    /// 賛成
    pub a: u32,
    /// 反対
    pub d: u32,
    /// 見た（パスを含む投票数）
    pub s: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseClusterOut {
    pub id: i64,
    pub count: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupOut {
    pub id: i64,
    /// 属する基底クラスタの id
    pub base_ids: Vec<i64>,
    /// 参加者数
    pub n_members: usize,
    /// 意見ごとの票（raw-rating-mat から集計）
    pub votes: HashMap<String, VoteCounts>,
}

/// 公開してよい計算結果（参加者 ID を含まない）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvResult {
    pub n_participants: usize,
    pub n_in_conv: usize,
    pub tids: Vec<String>,
    pub pca: Pca,
    pub comment_projection: Vec<[f64; 2]>,
    pub comment_extremity: Vec<f64>,
    pub base_clusters: Vec<BaseClusterOut>,
    pub groups: Vec<GroupOut>,
    pub repness: BTreeMap<i64, Vec<RepComment>>,
    pub group_aware_consensus: HashMap<String, f64>,
    pub consensus: Consensus,
    pub comment_priorities: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// 参加者か意見が2未満で、計算できない
    Insufficient {
        participants: usize,
        statements: usize,
    },
    Ready {
        result: Box<ConvResult>,
        state: ConvState,
    },
}

/// 計算に必要な最小の参加者数・意見数
pub const MIN_PARTICIPANTS: usize = 2;
pub const MIN_STATEMENTS: usize = 2;

/// グループ数の上限（`max-k-fn`）
pub fn max_k(n_base_clusters: usize) -> usize {
    MAX_K.min(2 + n_base_clusters / 12)
}

/// 表示順の優先度の元になる重要度（`importance-metric`）
pub fn importance_metric(a: f64, p: f64, s: f64, e: f64) -> f64 {
    let p = (p + 1.0) / (s + 2.0);
    let a = (a + 1.0) / (s + 2.0);
    (1.0 - p) * (e + 1.0) * a
}

/// 表示順の優先度（`priority-metric`）。新しい意見ほど上がるよう補正し、2乗する
pub fn priority_metric(is_meta: bool, a: f64, p: f64, s: f64, e: f64) -> f64 {
    let base = if is_meta {
        META_PRIORITY
    } else {
        importance_metric(a, p, s, e) * (1.0 + 8.0 * 2f64.powf(s / -5.0))
    };
    base.powi(2)
}

/// group-k-smoother: 同じ k が `GROUP_K_BUFFER` 回続いたら切り替える
fn smooth_k(
    prev: Option<KSmoother>,
    this_k: usize,
    available: &BTreeMap<usize, Vec<Cluster<i64>>>,
) -> KSmoother {
    let (same, prev_count, prev_smoothed) = match prev {
        Some(p) => (p.last_k == this_k, p.last_k_count, Some(p.smoothed_k)),
        None => (false, 0, None),
    };
    let count = if same { prev_count + 1 } else { 1 };
    let smoothed = if count >= GROUP_K_BUFFER {
        this_k
    } else {
        prev_smoothed.unwrap_or(this_k)
    };
    // clamp: 前回の k が今回は存在しない（基底クラスタが減った）ときは、今回の最良の k
    let smoothed = if available.contains_key(&smoothed) {
        smoothed
    } else {
        this_k
    };
    KSmoother {
        last_k: this_k,
        last_k_count: count,
        smoothed_k: smoothed,
    }
}

/// 前回の状態と全投票（時刻の古い順）から、分析を更新する
pub fn update(prev: &ConvState, votes: &[Vote], mod_out: &HashSet<String>) -> Outcome {
    // raw-rating-mat（前回の並びを引き継ぎ、新しい参加者・意見は末尾へ）
    let mut raw = NamedMatrix::with_order(prev.row_order.clone(), prev.col_order.clone());
    raw.update(
        votes
            .iter()
            .map(|v| (v.pid.as_str(), v.tid.as_str(), v.value)),
    );
    let rating = raw.zero_out_columns(mod_out);

    let n = rating.rownames().len();
    let n_cmts = rating.colnames().len();
    if n < MIN_PARTICIPANTS || n_cmts < MIN_STATEMENTS {
        return Outcome::Insufficient {
            participants: n,
            statements: n_cmts,
        };
    }

    // in-conv
    let user_vote_counts = raw.row_vote_counts();
    let mut in_conv: HashSet<String> = prev.in_conv.iter().cloned().collect();
    let threshold = IN_CONV_MIN_VOTES.min(n_cmts);
    for (pid, cnt) in &user_vote_counts {
        if *cnt >= threshold {
            in_conv.insert(pid.clone());
        }
    }
    if in_conv.len() < GREEDY_N {
        let mut rest: Vec<&(String, usize)> = user_vote_counts
            .iter()
            .filter(|(p, _)| !in_conv.contains(p))
            .collect();
        rest.sort_by_key(|x| std::cmp::Reverse(x.1)); // 安定ソート（同数なら行の順）
        let need = GREEDY_N - in_conv.len();
        for (pid, _) in rest.into_iter().take(need) {
            in_conv.insert(pid.clone());
        }
    }

    // 列平均で欠損を補った行列
    let rows = rating.rows();
    let col_avgs: Vec<f64> = (0..n_cmts)
        .map(|j| {
            let vals: Vec<f64> = rows.iter().filter_map(|r| r[j]).collect();
            if vals.is_empty() {
                0.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            }
        })
        .collect();
    let mat: Vec<Vec<f64>> = rows
        .iter()
        .map(|r| {
            r.iter()
                .enumerate()
                .map(|(j, x)| x.unwrap_or(col_avgs[j]))
                .collect()
        })
        .collect();

    // PCA と射影
    let start = prev
        .pca
        .as_ref()
        .map(|p| p.comps.clone())
        .unwrap_or_default();
    let pca = powerit_pca(&mat, N_COMPS, PCA_ITERS, &start);
    let comment_projection = pca_project_cmnts(&pca);
    let comment_extremity = comment_extremity(&comment_projection);
    let proj = sparsity_aware_project_ptpts(rows, &pca);

    // 基底クラスタ（in-conv の参加者だけ、行の順で）
    let (in_ids, in_coords): (Vec<String>, Vec<Vec<f64>>) = rating
        .rownames()
        .iter()
        .zip(&proj)
        .filter(|(pid, _)| in_conv.contains(*pid))
        .map(|(pid, p)| (pid.clone(), p.to_vec()))
        .unzip();
    let base_points = Points {
        ids: &in_ids,
        coords: &in_coords,
    };
    let last_base = (!prev.base_clusters.is_empty()).then_some(prev.base_clusters.as_slice());
    let base_clusters = kmeans(&base_points, BASE_K, last_base, BASE_ITERS, None);

    // グループ（基底クラスタの中心を、人数で重み付けしてクラスタリング）
    let base_ids: Vec<i64> = base_clusters.iter().map(|c| c.id).collect();
    let base_centers: Vec<Vec<f64>> = base_clusters.iter().map(|c| c.center.clone()).collect();
    let weights: HashMap<i64, f64> = base_clusters
        .iter()
        .map(|c| (c.id, c.members.len() as f64))
        .collect();
    let group_points = Points {
        ids: &base_ids,
        coords: &base_centers,
    };
    let bucket_dists = dist_matrix(&group_points);

    let mut group_clusterings: BTreeMap<usize, Vec<Cluster<i64>>> = BTreeMap::new();
    let mut silhouettes: BTreeMap<usize, f64> = BTreeMap::new();
    for k in 2..=max_k(base_clusters.len()) {
        let last = prev.group_clusterings.get(&k).map(Vec::as_slice);
        let clustering = kmeans(&group_points, k, last, GROUP_ITERS, Some(&weights));
        silhouettes.insert(k, silhouette(&base_ids, &bucket_dists, &clustering));
        group_clusterings.insert(k, clustering);
    }
    // シルエット係数が最大の k（同点なら大きい k。max-key は後のものを返す）
    let this_k = silhouettes
        .iter()
        .fold(None::<(usize, f64)>, |best, (&k, &s)| match best {
            Some((_, b)) if s < b => best,
            _ => Some((k, s)),
        })
        .map(|(k, _)| k)
        .expect("k は 2 以上");
    let smoother = smooth_k(prev.group_k_smoother, this_k, &group_clusterings);
    let group_clusters = group_clusterings[&smoother.smoothed_k].clone();

    // votes-base / group-votes（raw-rating-mat から。in-conv の参加者だけが数えられる）
    let tids: Vec<String> = rating.colnames().to_vec();
    let base_members: HashMap<i64, &Vec<String>> =
        base_clusters.iter().map(|c| (c.id, &c.members)).collect();
    let groups: Vec<GroupOut> = group_clusters
        .iter()
        .map(|g| {
            let pids: Vec<&String> = g
                .members
                .iter()
                .flat_map(|bid| base_members.get(bid).into_iter().flat_map(|m| m.iter()))
                .collect();
            let votes = tids
                .iter()
                .enumerate()
                .map(|(j, tid)| {
                    let mut c = VoteCounts::default();
                    for pid in &pids {
                        if let Some(v) = raw.row_by_name(pid).and_then(|r| r[j]) {
                            c.s += 1;
                            if v < 0.0 {
                                c.a += 1;
                            } else if v > 0.0 {
                                c.d += 1;
                            }
                        }
                    }
                    (tid.clone(), c)
                })
                .collect();
            GroupOut {
                id: g.id,
                base_ids: g.members.clone(),
                n_members: pids.len(),
                votes,
            }
        })
        .collect();

    // group-aware consensus: グループごとの (A+1)/(S+2) の積
    let group_aware_consensus: HashMap<String, f64> = tids
        .iter()
        .map(|tid| {
            let product = groups
                .iter()
                .map(|g| {
                    let c = g.votes[tid];
                    (f64::from(c.a) + 1.0) / (f64::from(c.s) + 2.0)
                })
                .product();
            (tid.clone(), product)
        })
        .collect();

    // 表示順の優先度（今回の group-votes から。Q2 の修正）
    let comment_priorities: HashMap<String, f64> = tids
        .iter()
        .enumerate()
        .map(|(j, tid)| {
            let (a, d, s) = groups.iter().fold((0.0, 0.0, 0.0), |(a, d, s), g| {
                let c = g.votes[tid];
                (a + f64::from(c.a), d + f64::from(c.d), s + f64::from(c.s))
            });
            let p = s - (a + d);
            let e = comment_extremity.get(j).copied().unwrap_or(0.0);
            (tid.clone(), priority_metric(false, a, p, s, e))
        })
        .collect();

    // 代表意見（rating-mat、つまり mod-out を 0 埋めした行列から）
    let group_columns: Vec<Vec<Vec<Option<f64>>>> = group_clusters
        .iter()
        .map(|g| {
            let member_rows: Vec<&[Option<f64>]> = g
                .members
                .iter()
                .flat_map(|bid| base_members.get(bid).into_iter().flat_map(|m| m.iter()))
                .filter_map(|pid| rating.row_by_name(pid))
                .collect();
            (0..n_cmts)
                .map(|j| member_rows.iter().map(|r| r[j]).collect())
                .collect()
        })
        .collect();
    let rep_stats = conv_repness(&group_columns);
    let reps = select_rep_comments(&tids, &rep_stats, group_clusters.len(), mod_out);
    let repness: BTreeMap<i64, Vec<RepComment>> =
        group_clusters.iter().map(|g| g.id).zip(reps).collect();

    // 全体の合意点（全参加者）
    let all_columns: Vec<Vec<Option<f64>>> = (0..n_cmts)
        .map(|j| rows.iter().map(|r| r[j]).collect())
        .collect();
    let consensus = select_consensus_comments(&tids, &all_columns, mod_out);

    let result = ConvResult {
        n_participants: n,
        n_in_conv: in_conv.len(),
        tids: tids.clone(),
        pca: pca.clone(),
        comment_projection,
        comment_extremity,
        base_clusters: base_clusters
            .iter()
            .map(|c| BaseClusterOut {
                id: c.id,
                count: c.members.len(),
                x: c.center[0],
                y: c.center[1],
            })
            .collect(),
        groups,
        repness,
        group_aware_consensus,
        consensus,
        comment_priorities,
    };
    let state = ConvState {
        row_order: rating.rownames().to_vec(),
        col_order: tids,
        in_conv: rating
            .rownames()
            .iter()
            .filter(|p| in_conv.contains(*p))
            .cloned()
            .collect(),
        pca: Some(pca),
        base_clusters,
        group_clusterings,
        group_k_smoother: Some(smoother),
    };
    Outcome::Ready {
        result: Box::new(result),
        state,
    }
}

/// 保存済みの主成分で、ある参加者の位置を計算する（`votes` は列の並びに合わせた Polis の符号の票）
pub fn project(pca: &Pca, votes: &[Option<f64>]) -> [f64; 2] {
    sparsity_aware_project_ptpt(votes, pca)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vote(pid: &str, tid: &str, value: f64) -> Vote {
        Vote {
            pid: pid.into(),
            tid: tid.into(),
            value,
        }
    }

    /// 賛成派 10 人と反対派 10 人に分かれる会話
    fn polarized() -> Vec<Vote> {
        let mut votes = Vec::new();
        for i in 0..20 {
            let side = if i < 10 { -1.0 } else { 1.0 };
            let pid = format!("p{i:02}");
            for t in 0..6 {
                votes.push(vote(&pid, &format!("t{t}"), side));
            }
            // 全員が賛成する意見
            votes.push(vote(&pid, "common", -1.0));
        }
        votes
    }

    #[test]
    fn insufficient_data_is_reported() {
        let out = update(
            &ConvState::default(),
            &[vote("p1", "t1", -1.0)],
            &HashSet::new(),
        );
        assert_eq!(
            out,
            Outcome::Insufficient {
                participants: 1,
                statements: 1
            }
        );
    }

    #[test]
    fn polarized_conversation_has_two_groups_and_a_consensus() {
        let Outcome::Ready { result, state } =
            update(&ConvState::default(), &polarized(), &HashSet::new())
        else {
            panic!("計算できるはず");
        };
        assert_eq!(result.groups.len(), 2);
        let mut sizes: Vec<usize> = result.groups.iter().map(|g| g.n_members).collect();
        sizes.sort();
        assert_eq!(sizes, [10, 10]);
        // 全員が賛成した意見は、全体の合意点に入る
        assert!(result.consensus.agree.iter().any(|c| c.tid == "common"));
        // どのグループでも (A+1)/(S+2) = 11/12
        let gac = result.group_aware_consensus["common"];
        assert!((gac - (11.0f64 / 12.0).powi(2)).abs() < 1e-12);
        // 状態には in-conv（全員）と smoother が残る
        assert_eq!(state.in_conv.len(), 20);
        assert_eq!(state.group_k_smoother.unwrap().last_k_count, 1);
    }

    #[test]
    fn update_is_deterministic_and_smoother_advances() {
        let votes = polarized();
        let Outcome::Ready {
            result: r1,
            state: s1,
        } = update(&ConvState::default(), &votes, &HashSet::new())
        else {
            panic!()
        };
        let Outcome::Ready { result: r1b, .. } =
            update(&ConvState::default(), &votes, &HashSet::new())
        else {
            panic!()
        };
        assert_eq!(
            r1, r1b,
            "同じ入力なら同じ結果（初期ベクトルを固定しているため）"
        );
        let Outcome::Ready { state: s2, .. } = update(&s1, &votes, &HashSet::new()) else {
            panic!()
        };
        assert_eq!(s2.group_k_smoother.unwrap().last_k_count, 2);
    }

    #[test]
    fn mod_out_statements_are_excluded_from_selection() {
        let mod_out = HashSet::from(["common".to_owned()]);
        let Outcome::Ready { result, .. } = update(&ConvState::default(), &polarized(), &mod_out)
        else {
            panic!()
        };
        assert!(result.consensus.agree.iter().all(|c| c.tid != "common"));
        assert!(result.repness.values().flatten().all(|c| c.tid != "common"));
    }

    #[test]
    fn priority_metric_matches_polis_examples() {
        // conversation.clj の comment ブロックにある試算値と同じ式
        let p = priority_metric(false, 1.0, 0.0, 1.0, 0.0);
        let expected =
            ((1.0 - 1.0 / 3.0) * 1.0 * (2.0 / 3.0) * (1.0 + 8.0 * 2f64.powf(-0.2))).powi(2);
        assert!((p - expected).abs() < 1e-12);
        assert_eq!(priority_metric(true, 0.0, 0.0, 0.0, 0.0), 49.0);
    }

    #[test]
    fn max_k_grows_with_base_clusters() {
        assert_eq!(max_k(1), 2);
        assert_eq!(max_k(12), 3);
        assert_eq!(max_k(100), 5);
    }

    #[test]
    fn smoother_switches_after_buffer() {
        let available: BTreeMap<usize, Vec<Cluster<i64>>> =
            [(2, vec![]), (3, vec![])].into_iter().collect();
        let mut s = smooth_k(None, 2, &available);
        assert_eq!(s.smoothed_k, 2);
        for i in 1..=4 {
            s = smooth_k(Some(s), 3, &available);
            let expected = if i >= 4 { 3 } else { 2 };
            assert_eq!(s.smoothed_k, expected, "{i} 回目");
        }
    }
}
