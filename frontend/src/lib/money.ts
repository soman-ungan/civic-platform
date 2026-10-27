// SPDX-License-Identifier: AGPL-3.0-only

// まちのお金の表示。backend の金額は千円、1人あたりは円

/** 円 → 「12,345円」 */
export function yen(value: number): string {
  return `${Math.round(value).toLocaleString("ja-JP")}円`;
}

/** 円 → 「約20.1万円」（1万円未満は円のまま） */
export function roughYen(value: number): string {
  if (Math.abs(value) < 10_000) return yen(value);
  const man = value / 10_000;
  const digits = Math.abs(man) < 100 ? 1 : 0;
  return `約${man.toLocaleString("ja-JP", { maximumFractionDigits: digits })}万円`;
}

/** 千円 → 「336.9億円」「5,000万円」 */
export function thousandYen(value: number): string {
  const oku = value / 100_000;
  if (Math.abs(oku) >= 1) {
    return `${oku.toLocaleString("ja-JP", { maximumFractionDigits: 1 })}億円`;
  }
  return `${Math.round(value / 10).toLocaleString("ja-JP")}万円`;
}

/** 0〜1 → 「37.8%」 */
export function shareText(share: number): string {
  return `${(share * 100).toLocaleString("ja-JP", { maximumFractionDigits: 1 })}%`;
}

/** 2023 → 「令和5年度」 */
export function fiscalYearLabel(year: number): string {
  if (year >= 2019) {
    const n = year - 2018;
    return `令和${n === 1 ? "元" : n}年度`;
  }
  return `平成${year - 1988}年度`;
}

/** 決算カードの出典の表示（公共データ利用規約（第1.0版）の「加工して作成」） */
export const SOURCE_NOTE = "出典：総務省「市町村決算カード」を加工して作成";
