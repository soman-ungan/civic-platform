// SPDX-License-Identifier: AGPL-3.0-only

// 議員の記録の表記（ADR 0008）

import { formatDate } from "@/lib/dates";
import type {
  Coverage,
  Official,
  OfficialRole,
} from "@/lib/api/types/officials";

/**
 * 役職（「市長」「町長」「議員」）。首長は自治体名の最後の字（市・町・村・区）から作る（自治体を書き込まない）
 */
export function roleLabel(
  municipalityName: string,
  role: OfficialRole,
): string {
  return role === "mayor" ? `${municipalityName.slice(-1)}長` : "議員";
}

/** 役職の正式な書き方（「大田原市長」「大田原市議会議員」） */
export function roleTitle(
  municipalityName: string,
  role: OfficialRole,
): string {
  return role === "mayor"
    ? `${municipalityName}長`
    : `${municipalityName}議会議員`;
}

/** 任期（「2023年11月19日から」「2023年11月19日〜2026年2月20日」） */
export function termLabel(
  o: Pick<Official, "term_start" | "term_end">,
): string {
  return o.term_end
    ? `${formatDate(o.term_start)}〜${formatDate(o.term_end)}`
    : `${formatDate(o.term_start)}から`;
}

/** 取り込んだ会議録の範囲の説明。記録が無いのは「発言していない」のではなく「載っていない」だけだと添える */
export function coverageText(coverage: Coverage | null): string {
  if (!coverage) {
    return "会議録はまだ取り込んでいません。";
  }
  return `このサービスに取り込んだ会議録（${formatDate(coverage.from)}〜${formatDate(coverage.to)}の会議）から作ったトピックだけを載せています。取り込んでいない会議での発言は載っていません。`;
}
