-- SPDX-License-Identifier: AGPL-3.0-only
-- 会議録から作った最初の意見の根拠（議会での発言）。運営者の確認用で、API では返さない
-- （意見は誰が言ったかではなく中身で判断してもらうため。docs/plans/2026-10-27-phase5a-minutes.md）
CREATE TABLE statement_sources (
    statement_id  TEXT NOT NULL REFERENCES statements (id),
    document_id   TEXT NOT NULL REFERENCES documents (id),
    utterance_no  INTEGER NOT NULL,
    -- 公人は「〇〇議員」など、職員は職名だけ
    speaker_label TEXT NOT NULL,
    PRIMARY KEY (statement_id, utterance_no)
);
