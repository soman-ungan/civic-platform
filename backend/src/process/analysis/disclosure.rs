// SPDX-License-Identifier: AGPL-3.0-only

//! 分析の結果を公表するときの決まり（分析の API と提言で共通。ADR 0002）。
//! 少人数の集計は個人の投票そのものになるので出さない。関わり方の区分は二次秘匿する

/// グループを表示する最小の参加者数（分析に含まれる人数）。
/// 少人数ではグループの集計が個人の投票そのものになるため（Polis にはない、このプロジェクトの判断）
pub const MIN_DISPLAY_PARTICIPANTS: usize = 5;
/// 関わり方ごとの集計で、これ未満の人数のセルは出さない
pub const MIN_CELL_SIZE: i64 = 5;
/// 合意点（タイムラインのピン留め・提言）の条件: 全グループで (A+1)/(S+2) がこの値以上
pub const PIN_THRESHOLD: f64 = 0.8;

/// 意見グループの表示名（A, B, C…）。`index` は分析の結果のグループの並び
pub fn group_label(index: usize) -> String {
    char::from(b'A' + (index % 26) as u8).to_string()
}

/// 二次秘匿: どの区分を表示してよいかを返す（`counts` と同じ順の bool）。
/// 1. 5人未満の区分を隠す
/// 2. 隠した区分の合計が 1〜4 人なら、見えている区分のうち最も少ないものも隠す（引き算で逆算されないように）
/// 3. 合計が5人以上になるか、すべて隠れるまで 2 を繰り返す
pub fn suppress(counts: &[i64]) -> Vec<bool> {
    let mut visible: Vec<bool> = counts.iter().map(|c| *c >= MIN_CELL_SIZE).collect();
    loop {
        let hidden: i64 = counts
            .iter()
            .zip(&visible)
            .filter(|(_, v)| !**v)
            .map(|(c, _)| *c)
            .sum();
        if hidden == 0 || hidden >= MIN_CELL_SIZE {
            return visible;
        }
        let smallest = counts
            .iter()
            .enumerate()
            .filter(|(i, _)| visible[*i])
            .min_by_key(|(_, c)| **c)
            .map(|(i, _)| i);
        match smallest {
            Some(i) => visible[i] = false,
            None => return visible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::suppress;

    #[test]
    fn nothing_hidden() {
        assert_eq!(suppress(&[6, 5, 7]), [true, true, true]);
    }

    #[test]
    fn hidden_total_of_five_or_more_is_safe() {
        // 3 + 2 = 5 人を隠すだけでよい
        assert_eq!(suppress(&[3, 2, 9]), [false, false, true]);
    }

    #[test]
    fn small_hidden_total_hides_next_smallest() {
        // 3 人だけ隠すと逆算できるので、次に少ない 5 も隠す
        assert_eq!(suppress(&[6, 5, 3, 0]), [true, false, false, false]);
    }

    #[test]
    fn may_hide_everything() {
        assert_eq!(suppress(&[7, 1]), [false, false]);
        assert_eq!(suppress(&[0, 0]), [false, false]);
    }
}
