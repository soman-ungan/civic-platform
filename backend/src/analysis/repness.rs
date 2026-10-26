// SPDX-License-Identifier: AGPL-3.0-only
//
// Ported from Polis (https://github.com/compdemocracy/polis)
//   math/src/polismath/math/repness.clj @ b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9
// Copyright (C) 2012-present, The Authors (Polis).
// Copyright (C) 2026 soman-ungan (Rust port).
// Licensed under the GNU Affero General Public License, version 3.

//! グループの代表意見（repness）と、全体の合意点（consensus）の選び方（Polis の `repness.clj`）。
//! 投票の値は Polis の符号（賛成 = -1、反対 = 1）。

use serde::{Deserialize, Serialize};

use super::stats::{prop_test, two_prop_test, z_sig_90};

/// ある意見・あるグループの集計（n=数、p=確率、t=検定、a=賛成、d=反対、s=見た）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommentStats {
    pub na: f64,
    pub nd: f64,
    pub ns: f64,
    pub pa: f64,
    pub pd: f64,
    pub pat: f64,
    pub pdt: f64,
}

/// グループ内とそれ以外を比べた値を加えたもの
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComparativeStats {
    pub base: CommentStats,
    pub ra: f64,
    pub rd: f64,
    pub rat: f64,
    pub rdt: f64,
}

/// 1列（ある意見へのグループの票）の集計（`comment-stats`）。
/// `ns` は未投票以外をすべて数える（0 も数える。Clojure では 0 が真になるため）
pub fn comment_stats<'a>(votes: impl IntoIterator<Item = &'a Option<f64>>) -> CommentStats {
    let (mut na, mut nd, mut ns) = (0.0, 0.0, 0.0);
    for v in votes.into_iter().flatten() {
        ns += 1.0;
        if *v == -1.0 {
            na += 1.0;
        } else if *v == 1.0 {
            nd += 1.0;
        }
    }
    CommentStats {
        na,
        nd,
        ns,
        pa: (1.0 + na) / (2.0 + ns),
        pd: (1.0 + nd) / (2.0 + ns),
        pat: prop_test(na, ns),
        pdt: prop_test(nd, ns),
    }
}

fn add_comparative_stats(own: CommentStats, rest: &[CommentStats]) -> ComparativeStats {
    let rest_na: f64 = rest.iter().map(|s| s.na).sum();
    let rest_nd: f64 = rest.iter().map(|s| s.nd).sum();
    let rest_ns: f64 = rest.iter().map(|s| s.ns).sum();
    ComparativeStats {
        base: own,
        ra: own.pa / ((1.0 + rest_na) / (2.0 + rest_ns)),
        rd: own.pd / ((1.0 + rest_nd) / (2.0 + rest_ns)),
        rat: two_prop_test(own.na, rest_na, own.ns, rest_ns),
        rdt: two_prop_test(own.nd, rest_nd, own.ns, rest_ns),
    }
}

/// 全意見 × 全グループの比較つき集計（`conv-repness`）。
/// `group_columns[g][t]` = グループ g の、意見 t への票の列。戻り値は `[t][g]`
pub fn conv_repness(group_columns: &[Vec<Vec<Option<f64>>>]) -> Vec<Vec<ComparativeStats>> {
    let n_tids = group_columns.first().map_or(0, Vec::len);
    (0..n_tids)
        .map(|t| {
            let per_group: Vec<CommentStats> = group_columns
                .iter()
                .map(|cols| comment_stats(&cols[t]))
                .collect();
            (0..per_group.len())
                .map(|g| {
                    let rest: Vec<CommentStats> = per_group
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != g)
                        .map(|(_, s)| *s)
                        .collect();
                    add_comparative_stats(per_group[g], &rest)
                })
                .collect()
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepfulFor {
    Agree,
    Disagree,
}

/// 代表意見として返す形（`finalize-cmt-stats`）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepComment {
    pub tid: String,
    pub n_success: f64,
    pub n_trials: f64,
    pub p_success: f64,
    pub p_test: f64,
    pub repness: f64,
    /// Polis と同じく float32 に丸めた値
    pub repness_test: f64,
    pub repful_for: RepfulFor,
    /// best-agree として先頭に入れたもの
    #[serde(default)]
    pub best_agree: bool,
}

fn finalize(tid: &str, s: &ComparativeStats) -> RepComment {
    let (n_success, p_success, p_test, repness, repness_test, repful_for) = if s.rat > s.rdt {
        (
            s.base.na,
            s.base.pa,
            s.base.pat,
            s.ra,
            s.rat,
            RepfulFor::Agree,
        )
    } else {
        (
            s.base.nd,
            s.base.pd,
            s.base.pdt,
            s.rd,
            s.rdt,
            RepfulFor::Disagree,
        )
    };
    RepComment {
        tid: tid.to_owned(),
        n_success,
        n_trials: s.base.ns,
        p_success,
        p_test,
        repness,
        repness_test: f64::from(repness_test as f32),
        repful_for,
        best_agree: false,
    }
}

fn repness_metric(c: &RepComment) -> f64 {
    c.repness * c.repness_test * c.p_success * c.p_test
}

fn passes_by_test(s: &ComparativeStats) -> bool {
    (z_sig_90(s.rat) && z_sig_90(s.base.pat)) || (z_sig_90(s.rdt) && z_sig_90(s.base.pdt))
}

fn beats_best_by_test(s: &ComparativeStats, current_best_z: Option<f64>) -> bool {
    current_best_z.is_none_or(|z| s.rat.max(s.rdt) > z)
}

fn beats_best_agr(s: &ComparativeStats, current: Option<&ComparativeStats>) -> bool {
    let b = &s.base;
    if b.na == 0.0 && b.nd == 0.0 {
        return false;
    }
    match current {
        Some(c) if c.ra > 1.0 => {
            s.ra * s.rat * b.pa * b.pat > c.ra * c.rat * c.base.pa * c.base.pat
        }
        Some(c) => b.pa * b.pat > c.base.pa * c.base.pat,
        None => z_sig_90(b.pat) || (s.ra > 1.0 && b.pa > 0.5),
    }
}

/// 各グループの代表意見を最大5件選ぶ（`select-rep-comments`）。
/// `stats[t][g]` は `conv_repness` の結果。`mod_out` の意見は除く。戻り値は `[g]`
pub fn select_rep_comments(
    tids: &[String],
    stats: &[Vec<ComparativeStats>],
    n_groups: usize,
    mod_out: &std::collections::HashSet<String>,
) -> Vec<Vec<RepComment>> {
    struct Acc<'a> {
        best: Option<RepComment>,
        best_agree: Option<(&'a str, ComparativeStats)>,
        sufficient: Vec<RepComment>,
    }
    let mut acc: Vec<Acc> = (0..n_groups)
        .map(|_| Acc {
            best: None,
            best_agree: None,
            sufficient: Vec::new(),
        })
        .collect();

    for (tid, per_group) in tids.iter().zip(stats) {
        if mod_out.contains(tid) {
            continue;
        }
        for (g, s) in per_group.iter().enumerate() {
            let a = &mut acc[g];
            if passes_by_test(s) {
                a.sufficient.push(finalize(tid, s));
            }
            if a.sufficient.is_empty()
                && beats_best_by_test(s, a.best.as_ref().map(|b| b.repness_test))
            {
                a.best = Some(finalize(tid, s));
            }
            if beats_best_agr(s, a.best_agree.as_ref().map(|(_, c)| c)) {
                a.best_agree = Some((tid.as_str(), *s));
            }
        }
    }

    acc.into_iter()
        .map(|a| {
            let best_agree = a.best_agree.map(|(tid, s)| RepComment {
                best_agree: true,
                ..finalize(tid, &s)
            });
            if a.sufficient.is_empty() {
                return best_agree.or(a.best).into_iter().collect();
            }
            let mut sufficient: Vec<RepComment> = a
                .sufficient
                .into_iter()
                .filter(|c| best_agree.as_ref().is_none_or(|b| b.tid != c.tid))
                .collect();
            // repness-metric の大きい順（安定ソート）
            sufficient.sort_by(|x, y| {
                repness_metric(y)
                    .partial_cmp(&repness_metric(x))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let top: Vec<RepComment> = best_agree.into_iter().chain(sufficient).take(5).collect();
            // 賛成を先、反対を後に
            let (agrees, disagrees): (Vec<_>, Vec<_>) = top
                .into_iter()
                .partition(|c| c.repful_for == RepfulFor::Agree);
            agrees.into_iter().chain(disagrees).collect()
        })
        .collect()
}

/// 合意点として返す形
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusComment {
    pub tid: String,
    pub n_success: f64,
    pub n_trials: f64,
    pub p_success: f64,
    pub p_test: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Consensus {
    pub agree: Vec<ConsensusComment>,
    pub disagree: Vec<ConsensusComment>,
}

/// 全体の合意点（`consensus-stats` + `select-consensus-comments`）。
/// `columns[t]` は意見 t への全参加者の票。確率 > 0.5 かつ 90% で有意なものの上位5件
pub fn select_consensus_comments(
    tids: &[String],
    columns: &[Vec<Option<f64>>],
    mod_out: &std::collections::HashSet<String>,
) -> Consensus {
    let stats: Vec<(&String, CommentStats)> = tids
        .iter()
        .zip(columns)
        .filter(|(tid, _)| !mod_out.contains(*tid))
        .map(|(tid, col)| (tid, comment_stats(col)))
        .collect();

    let top5 = |agree: bool| -> Vec<ConsensusComment> {
        let mut candidates: Vec<(f64, ConsensusComment)> = stats
            .iter()
            .filter_map(|(tid, s)| {
                let (n, p, t) = if agree {
                    (s.na, s.pa, s.pat)
                } else {
                    (s.nd, s.pd, s.pdt)
                };
                (p > 0.5 && z_sig_90(t)).then(|| {
                    (
                        p * t,
                        ConsensusComment {
                            tid: (*tid).clone(),
                            n_success: n,
                            n_trials: s.ns,
                            p_success: p,
                            p_test: t,
                        },
                    )
                })
            })
            .collect();
        candidates.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
        candidates.into_iter().take(5).map(|(_, c)| c).collect()
    };

    Consensus {
        agree: top5(true),
        disagree: top5(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn col(values: &[Option<f64>]) -> Vec<Option<f64>> {
        values.to_vec()
    }

    #[test]
    fn comment_stats_counts_passes_as_seen() {
        let s = comment_stats(&col(&[Some(-1.0), Some(-1.0), Some(1.0), Some(0.0), None]));
        assert_eq!((s.na, s.nd, s.ns), (2.0, 1.0, 4.0));
        assert!((s.pa - 3.0 / 6.0).abs() < 1e-12);
        assert!((s.pd - 2.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn group_that_agrees_gets_agree_repness() {
        // グループ0 は意見0 に全員賛成、グループ1 は全員反対
        let g0 = vec![col(&[Some(-1.0); 10]), col(&[Some(0.0); 10])];
        let g1 = vec![col(&[Some(1.0); 10]), col(&[Some(0.0); 10])];
        let stats = conv_repness(&[g0, g1]);
        let tids = vec!["t0".to_owned(), "t1".to_owned()];
        let reps = select_rep_comments(&tids, &stats, 2, &HashSet::new());
        assert_eq!(reps[0][0].tid, "t0");
        assert_eq!(reps[0][0].repful_for, RepfulFor::Agree);
        assert_eq!(reps[1][0].tid, "t0");
        assert_eq!(reps[1][0].repful_for, RepfulFor::Disagree);
    }

    #[test]
    fn moderated_comments_are_never_selected() {
        let g0 = vec![col(&[Some(-1.0); 10])];
        let g1 = vec![col(&[Some(1.0); 10])];
        let stats = conv_repness(&[g0, g1]);
        let tids = vec!["t0".to_owned()];
        let reps = select_rep_comments(&tids, &stats, 2, &HashSet::from(["t0".to_owned()]));
        assert!(reps.iter().all(Vec::is_empty));
    }

    #[test]
    fn consensus_requires_majority_and_significance() {
        let tids = vec!["t0".to_owned(), "t1".to_owned(), "t2".to_owned()];
        let columns = vec![
            col(&[Some(-1.0); 20]),                               // 全員賛成
            col(&[Some(-1.0), Some(1.0), Some(-1.0), Some(1.0)]), // 割れている
            col(&[Some(1.0); 20]),                                // 全員反対
        ];
        let c = select_consensus_comments(&tids, &columns, &HashSet::new());
        assert_eq!(
            c.agree.iter().map(|x| x.tid.as_str()).collect::<Vec<_>>(),
            ["t0"]
        );
        assert_eq!(
            c.disagree
                .iter()
                .map(|x| x.tid.as_str())
                .collect::<Vec<_>>(),
            ["t2"]
        );
        assert_eq!(c.agree[0].n_success, 20.0);
    }
}
