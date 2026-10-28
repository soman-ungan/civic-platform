-- SPDX-License-Identifier: AGPL-3.0-only
-- AI に読ませた資料の、出力の作り方（docs/plans/2026-10-28-manual-extraction.md）。
-- api: Claude API / manual: 運営者が共同作業（Claude Code など）で作った出力。
-- AI を使わない資料（決算カード）と、これより前に取り込んだものは NULL
ALTER TABLE documents ADD COLUMN extraction TEXT CHECK (extraction IN ('api', 'manual'));
