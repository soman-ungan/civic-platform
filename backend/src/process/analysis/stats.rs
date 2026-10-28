// SPDX-License-Identifier: AGPL-3.0-only
//
// Ported from Polis (https://github.com/compdemocracy/polis)
//   math/src/polismath/math/stats.clj @ b4971eee61e4c8efa3d0f47f9b2a8bee3c9c50f9
// Copyright (C) 2012-present, The Authors (Polis).
// Copyright (C) 2026 soman-ungan (Rust port).
// Licensed under the GNU Affero General Public License, version 3.

//! 比率の検定（Polis の `stats.clj`）。

/// 片側の比率検定。成功数・試行数にそれぞれ 1 を足してから z 値を計算する。
pub fn prop_test(succ: f64, n: f64) -> f64 {
    let (succ, n) = (succ + 1.0, n + 1.0);
    2.0 * n.sqrt() * (succ / n - 0.5)
}

/// 2群の比率の差の検定。各値に 1 を足してから計算する。
/// 合算した比率が 1 のときは 0 を返す（Polis と同じ）。
pub fn two_prop_test(succ_in: f64, succ_out: f64, pop_in: f64, pop_out: f64) -> f64 {
    let (succ_in, succ_out, pop_in, pop_out) =
        (succ_in + 1.0, succ_out + 1.0, pop_in + 1.0, pop_out + 1.0);
    let pi1 = succ_in / pop_in;
    let pi2 = succ_out / pop_out;
    let pi_hat = (succ_in + succ_out) / (pop_in + pop_out);
    if pi_hat == 1.0 {
        return 0.0;
    }
    (pi1 - pi2) / (pi_hat * (1.0 - pi_hat) * (1.0 / pop_in + 1.0 / pop_out)).sqrt()
}

/// 片側 90% で有意か
pub fn z_sig_90(z: f64) -> bool {
    z > 1.2816
}

/// 片側 95% で有意か
pub fn z_sig_95(z: f64) -> bool {
    z > 1.6449
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prop_test_matches_formula() {
        // succ=3, n=4 → (4/5 - 0.5) * 2 * sqrt(5)
        let expected = 2.0 * 5f64.sqrt() * (4.0 / 5.0 - 0.5);
        assert!((prop_test(3.0, 4.0) - expected).abs() < 1e-12);
        // 半々なら 0 に近い（擬似カウントのぶんだけずれる）
        assert!((prop_test(0.0, 0.0) - 2.0 * (1.0 / 1.0 - 0.5)).abs() < 1e-12);
    }

    #[test]
    fn two_prop_test_is_zero_when_pooled_ratio_is_one() {
        // 擬似カウント込みで pi_hat が 1 になるのは、全員が成功している場合
        assert_eq!(two_prop_test(3.0, 2.0, 3.0, 2.0), 0.0);
    }

    #[test]
    fn two_prop_test_sign() {
        assert!(two_prop_test(9.0, 1.0, 10.0, 10.0) > 0.0);
        assert!(two_prop_test(1.0, 9.0, 10.0, 10.0) < 0.0);
    }

    #[test]
    fn significance_thresholds() {
        assert!(z_sig_90(1.3) && !z_sig_90(1.28));
        assert!(z_sig_95(1.65) && !z_sig_95(1.6));
    }
}
