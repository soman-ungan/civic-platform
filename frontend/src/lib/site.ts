// SPDX-License-Identifier: AGPL-3.0-only

// このサービスについて・利用規約・プライバシーポリシーに載せる、運営者ごとの値。
// リポジトリは公開するので、個人の名前などはコードに書かず `.env` から読む（.env.example を参照）。
// ビルドのときではなく、起動したサーバーでリクエストのときに読む（イメージに焼き込まない。
// 値を変えても作り直さなくてよい）。ブラウザの JS には入れない

/** 項目と、その環境変数の名前 */
export const SITE_FIELDS = {
  /** 運営者の名前（個人名または屋号） */
  operatorName: "SITE_OPERATOR_NAME",
  /** 収支を公開するページの URL */
  financeReportUrl: "SITE_FINANCE_REPORT_URL",
  /** 第一審の専属的合意管轄裁判所（例: 宇都宮地方裁判所） */
  jurisdictionCourt: "SITE_JURISDICTION_COURT",
  /** サーバーのアクセスの記録を保存する期間（例: 90日） */
  accessLogRetention: "SITE_ACCESS_LOG_RETENTION",
  /** サーバーの事業者と国・地域（例: 〇〇株式会社（日本）） */
  hosting: "SITE_HOSTING",
  /** AI の事業者でのデータの扱い（保存の期間・学習に使わないことなど。確かめて書く） */
  aiDataHandling: "SITE_AI_DATA_HANDLING",
  /** 問い合わせのメールアドレス */
  contactEmail: "SITE_CONTACT_EMAIL",
} as const;

export type SiteField = keyof typeof SITE_FIELDS;

/** `.env` の値（空なら null）。呼んだときに読む */
export function siteValue(field: SiteField): string | null {
  const v = process.env[SITE_FIELDS[field]]?.trim();
  return v ? v : null;
}
