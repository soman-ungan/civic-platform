-- SPDX-License-Identifier: AGPL-3.0-only
-- LLM（Claude API）の呼び出しごとの使用量と推定費用（docs/plans/2026-10-27-phase4-ai.md）
-- 1日の費用の上限の判定と、運用費の把握に使う。送った本文や参加者の情報は記録しない

CREATE TABLE llm_usage (
    id                          TEXT PRIMARY KEY,
    purpose                     TEXT NOT NULL
                                CHECK (purpose IN ('moderation', 'tagging', 'eval', 'extraction')),
    model                       TEXT NOT NULL,
    input_tokens                INTEGER NOT NULL,
    output_tokens               INTEGER NOT NULL,
    cache_creation_input_tokens INTEGER NOT NULL,
    cache_read_input_tokens     INTEGER NOT NULL,
    -- 推定費用（100万分の1ドル単位の整数。浮動小数点の誤差を避けるため）
    cost_micro_usd              INTEGER NOT NULL,
    -- ok: 正常 / refusal: 安全上の理由で回答しなかった / max_tokens: 出力が途中で切れた / invalid: 応答を解釈できなかった
    outcome                     TEXT NOT NULL
                                CHECK (outcome IN ('ok', 'refusal', 'max_tokens', 'invalid')),
    created_at                  TEXT NOT NULL
);

CREATE INDEX idx_llm_usage_created ON llm_usage (created_at);
