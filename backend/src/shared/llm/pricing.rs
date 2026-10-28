// SPDX-License-Identifier: AGPL-3.0-only

//! 使用量から費用を見積もる。
//! 料金は Anthropic の公開料金（2026-10-06 時点、100万トークンあたりの米ドル）。
//! 改定されたら、ここを直す。

use super::Usage;

/// 100万トークンあたりの料金（米ドル）
struct Price {
    input: f64,
    output: f64,
    /// キャッシュからの読み込み
    cache_read: f64,
    /// キャッシュへの書き込み（5分の保持）
    cache_write: f64,
}

const OPUS_5_5: Price = Price {
    input: 4.0,
    output: 20.0,
    cache_read: 0.20,
    cache_write: 5.0,
};
const SONNET_5_5: Price = Price {
    input: 2.0,
    output: 10.0,
    cache_read: 0.20,
    cache_write: 2.5,
};
const HAIKU_5_5: Price = Price {
    input: 0.10,
    output: 0.50,
    cache_read: 0.01,
    cache_write: 0.125,
};
/// Haiku 5.5 は、入力が10万トークンを超えると高い料金になる
const HAIKU_5_5_LONG: Price = Price {
    input: 0.50,
    output: 2.50,
    cache_read: 0.05,
    cache_write: 0.625,
};
const HAIKU_LONG_INPUT: i64 = 100_000;

fn price_for(model: &str, usage: &Usage) -> &'static Price {
    let total_input =
        usage.input_tokens + usage.cache_creation_input_tokens + usage.cache_read_input_tokens;
    if model.starts_with("claude-haiku-5-5") {
        if total_input > HAIKU_LONG_INPUT {
            &HAIKU_5_5_LONG
        } else {
            &HAIKU_5_5
        }
    } else if model.starts_with("claude-sonnet-5-5") {
        &SONNET_5_5
    } else {
        if !model.starts_with("claude-opus-5-5") {
            // 料金表にないモデルは、少なく見積もらないよう高い料金で数える
            tracing::warn!(
                model,
                "料金表にないモデルです。Opus 5.5 の料金で見積もります"
            );
        }
        &OPUS_5_5
    }
}

/// 推定費用（100万分の1ドル）。1トークンあたりの100万分の1ドル = 100万トークンあたりのドル
pub fn cost_micro_usd(model: &str, usage: &Usage) -> i64 {
    let p = price_for(model, usage);
    let cost = usage.input_tokens as f64 * p.input
        + usage.output_tokens as f64 * p.output
        + usage.cache_read_input_tokens as f64 * p.cache_read
        + usage.cache_creation_input_tokens as f64 * p.cache_write;
    cost.ceil() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(input: i64, output: i64) -> Usage {
        Usage {
            input_tokens: input,
            output_tokens: output,
            ..Usage::default()
        }
    }

    #[test]
    fn sonnet() {
        // 入力 1,000 × $2 + 出力 100 × $10 = 3,000 マイクロドル（0.003 ドル）
        assert_eq!(
            cost_micro_usd("claude-sonnet-5-5", &usage(1_000, 100)),
            3_000
        );
    }

    #[test]
    fn haiku_long_input_uses_higher_tier() {
        assert_eq!(cost_micro_usd("claude-haiku-5-5", &usage(1_000, 100)), 150);
        assert_eq!(
            cost_micro_usd("claude-haiku-5-5", &usage(200_000, 0)),
            100_000
        );
    }

    #[test]
    fn cache_tokens() {
        let u = Usage {
            input_tokens: 10,
            output_tokens: 0,
            cache_creation_input_tokens: 1_000,
            cache_read_input_tokens: 1_000,
        };
        // 10×2 + 1000×2.5 + 1000×0.2
        assert_eq!(cost_micro_usd("claude-sonnet-5-5", &u), 2_720);
    }

    #[test]
    fn unknown_model_is_priced_high() {
        assert_eq!(cost_micro_usd("claude-unknown", &usage(1_000, 0)), 4_000);
    }
}
