-- SPDX-License-Identifier: AGPL-3.0-only
-- トピックごとの分析結果（Polis の数理処理。docs/polis-math.md）
-- result / state は JSON を TEXT で保持する（PostgreSQL 移行時に jsonb を検討）

CREATE TABLE topic_analyses (
    topic_id          TEXT PRIMARY KEY REFERENCES topics (id),
    -- 入力（投票数・最新の投票時刻・非公開の意見）が変わったかを判定する値
    input_fingerprint TEXT NOT NULL,
    status            TEXT NOT NULL CHECK (status IN ('ready', 'insufficient')),
    -- 公開してよい計算結果（参加者 ID を含まない）。insufficient のときは不足している数
    result            TEXT NOT NULL,
    -- 次回の計算に引き継ぐ状態（行と列の並び、in-conv、主成分、クラスタ、smoother）
    state             TEXT NOT NULL,
    computed_at       TEXT NOT NULL
);
