// SPDX-License-Identifier: AGPL-3.0-only

//! Polis の Clojure 実装との一致テスト。
//! 正解データ: tests/fixtures/polis-biodiversity/（CC BY 4.0。出典は同ディレクトリの README.md）
//! 比較の基準は Polis 自身の比較テスト（delphi/tests/test_legacy_clojure_regression.py）に合わせる。

use std::collections::{HashMap, HashSet};

use backend::analysis::conversation::{ConvState, Outcome, Vote, update};
use serde_json::Value;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/polis-biodiversity"
);

/// CSV を時刻順（同時刻はファイル順）に並べ、同じ参加者・意見は最新の1票だけを残す。
/// CSV の vote は賛成 = 1 なので、Polis の符号（賛成 = -1）に反転する。
fn load_votes() -> Vec<Vote> {
    let text = std::fs::read_to_string(format!(
        "{DIR}/2025-11-11-1704-r4tykwac8thvzv35jrn53-votes.csv"
    ))
    .unwrap();
    let mut rows: Vec<(i64, usize, String, String, f64)> = text
        .lines()
        .skip(1)
        .enumerate()
        .map(|(i, line)| {
            // timestamp,datetime,comment-id,voter-id,vote（datetime にカンマは含まれない）
            let f: Vec<&str> = line.split(',').collect();
            (
                f[0].parse().unwrap(),
                i,
                f[3].to_owned(),
                f[2].to_owned(),
                -f[4].parse::<f64>().unwrap(),
            )
        })
        .collect();
    rows.sort_by_key(|r| (r.0, r.1));
    // 最新の1票だけを残し、その票の位置に置く
    let mut last: HashMap<(String, String), usize> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        last.insert((r.2.clone(), r.3.clone()), i);
    }
    rows.iter()
        .enumerate()
        .filter(|(i, r)| last[&(r.2.clone(), r.3.clone())] == *i)
        .map(|(_, r)| Vote {
            pid: r.2.clone(),
            tid: r.3.clone(),
            value: r.4,
        })
        .collect()
}

fn load_blob() -> Value {
    let text = std::fs::read_to_string(format!(
        "{DIR}/r4tykwac8thvzv35jrn53_math_blob_cold_start.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.to_string().trim_matches('"').to_owned())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    a.intersection(b).count() as f64 / a.union(b).count() as f64
}

#[test]
fn matches_clojure_cold_start_on_biodiversity() {
    let votes = load_votes();
    let blob = load_blob();
    // 行（参加者）と列（意見）の並びを、Clojure の行列と同じ ID の昇順にする。
    // 並びは k-means の初期クラスタと、同点の並び順に影響する（docs/polis-math.md）
    let mut pids: Vec<String> = votes
        .iter()
        .map(|v| v.pid.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    pids.sort_by_key(|p| p.parse::<i64>().unwrap());
    let initial = ConvState {
        row_order: pids,
        col_order: ids(&blob["tids"]),
        ..ConvState::default()
    };
    let Outcome::Ready { result, state } = update(&initial, &votes, &HashSet::new()) else {
        panic!("計算できるはず");
    };

    // 1. 参加者数・意見数・in-conv
    assert_eq!(result.n_participants, blob["n"].as_u64().unwrap() as usize);
    assert_eq!(result.tids.len(), blob["n-cmts"].as_u64().unwrap() as usize);
    let blob_in_conv: HashSet<String> = ids(&blob["in-conv"]).into_iter().collect();
    let in_conv: HashSet<String> = state.in_conv.iter().cloned().collect();
    eprintln!(
        "in-conv: ours {} / clojure {}",
        in_conv.len(),
        blob_in_conv.len()
    );
    assert_eq!(in_conv, blob_in_conv, "in-conv は完全に一致する");

    // 2. 主成分（符号は不問。角度で比べる）
    let blob_tids = ids(&blob["tids"]);
    let pos: HashMap<&String, usize> = result
        .tids
        .iter()
        .enumerate()
        .map(|(i, t)| (t, i))
        .collect();
    let mut angles = Vec::new();
    for c in 0..2 {
        let clj: Vec<f64> = blob["pca"]["comps"][c]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let ours: Vec<f64> = blob_tids
            .iter()
            .map(|t| result.pca.comps[c][pos[t]])
            .collect();
        let dot: f64 = clj.iter().zip(&ours).map(|(a, b)| a * b).sum();
        let norm = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
        let cos = (dot / (norm(&clj) * norm(&ours))).abs().min(1.0);
        angles.push(cos.acos().to_degrees());
    }
    eprintln!("PCA angle: PC1 {:.4}°, PC2 {:.4}°", angles[0], angles[1]);
    for a in &angles {
        assert!(*a <= 10.0, "主成分の角度は 10° 以内（Polis の基準）");
    }

    // 3. グループ: 数、所属の Jaccard、人数分布
    let base = &blob["base-clusters"];
    let base_members: HashMap<i64, HashSet<String>> = base["id"]
        .as_array()
        .unwrap()
        .iter()
        .zip(base["members"].as_array().unwrap())
        .map(|(id, m)| (id.as_i64().unwrap(), ids(m).into_iter().collect()))
        .collect();
    let clj_groups: Vec<HashSet<String>> = blob["group-clusters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            g["members"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|b| base_members[&b.as_i64().unwrap()].iter().cloned())
                .collect()
        })
        .collect();
    // 自分のグループの参加者集合は、保存した基底クラスタから作る
    let our_base: HashMap<i64, HashSet<String>> = state
        .base_clusters
        .iter()
        .map(|c| (c.id, c.members.iter().cloned().collect()))
        .collect();
    let our_groups: Vec<HashSet<String>> = result
        .groups
        .iter()
        .map(|g| {
            g.base_ids
                .iter()
                .flat_map(|b| our_base[b].iter().cloned())
                .collect()
        })
        .collect();
    eprintln!(
        "groups: ours {:?} / clojure {:?}",
        our_groups.iter().map(HashSet::len).collect::<Vec<_>>(),
        clj_groups.iter().map(HashSet::len).collect::<Vec<_>>()
    );
    eprintln!(
        "base clusters: ours {} / clojure {}",
        state.base_clusters.len(),
        base_members.len()
    );
    assert_eq!(our_groups.len(), clj_groups.len(), "グループの数は一致する");
    let mut mapping = Vec::new();
    let mut jaccards = Vec::new();
    for g in &our_groups {
        let (best, j) = clj_groups
            .iter()
            .enumerate()
            .map(|(i, c)| (i, jaccard(g, c)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        mapping.push(best);
        jaccards.push(j);
    }
    let avg_j = jaccards.iter().sum::<f64>() / jaccards.len() as f64;
    let total: f64 = our_groups.iter().map(|g| g.len() as f64).sum();
    let l1: f64 = our_groups
        .iter()
        .zip(&mapping)
        .map(|(g, &m)| (g.len() as f64 / total - clj_groups[m].len() as f64 / total).abs())
        .sum();
    eprintln!("group Jaccard: {jaccards:?} (avg {avg_j:.4}), size L1 {l1:.4}");
    assert!(
        avg_j >= 0.99,
        "所属の Jaccard の平均は 0.99 以上（Polis の基準）"
    );
    assert!(l1 <= 0.01, "人数分布の差（L1）は 0.01 以内（Polis の基準）");
    // 4. 全体の合意点（クラスタリングに依存しないので完全一致を求める）
    for side in ["agree", "disagree"] {
        let clj: Vec<(String, f64, f64)> = blob["consensus"][side]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["tid"].to_string(),
                    c["n-success"].as_f64().unwrap(),
                    c["n-trials"].as_f64().unwrap(),
                )
            })
            .collect();
        let ours: Vec<(String, f64, f64)> = match side {
            "agree" => &result.consensus.agree,
            _ => &result.consensus.disagree,
        }
        .iter()
        .map(|c| (c.tid.clone(), c.n_success, c.n_trials))
        .collect();
        eprintln!(
            "consensus {side}: ours {:?}",
            ours.iter().map(|c| &c.0).collect::<Vec<_>>()
        );
        assert_eq!(ours, clj, "consensus ({side}) は完全に一致する");
    }

    // 5. group-aware consensus
    let max_diff = blob["group-aware-consensus"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(tid, v)| (result.group_aware_consensus[tid] - v.as_f64().unwrap()).abs())
        .fold(0.0, f64::max);
    eprintln!("group-aware consensus: max abs diff {max_diff:.3e}");
    // グループの所属が1人違う（Q11 / Q17 の同点処理を再現していないため）ぶんの差だけ許容する
    assert!(max_diff < 0.01, "group-aware consensus の差は 0.01 未満");

    // 6. 代表意見（Polis の Python 版の比較テストでは未一致として扱われている項目だが、ここでは一致する）
    for (our_idx, &clj_idx) in mapping.iter().enumerate() {
        let gid = result.groups[our_idx].id;
        let ours: Vec<String> = result.repness[&gid].iter().map(|c| c.tid.clone()).collect();
        let clj: Vec<String> = blob["repness"][clj_idx.to_string()]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["tid"].to_string())
            .collect();
        eprintln!("repness group {our_idx}: ours {ours:?} / clojure {clj:?}");
        assert_eq!(ours, clj, "代表意見の tid と並び順は一致する");
    }

    // 7. 表示順の優先度: 正解データは #1961 の不具合（全部 49）が入った状態で作られている
    let clj_priorities: HashSet<u64> = blob["comment-priorities"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_f64().unwrap().to_bits())
        .collect();
    assert_eq!(clj_priorities, HashSet::from([49f64.to_bits()]));
    let distinct: HashSet<u64> = result
        .comment_priorities
        .values()
        .map(|v| v.to_bits())
        .collect();
    assert!(distinct.len() > 1, "修正後の式では意見ごとに優先度が異なる");
}
