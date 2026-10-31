// SPDX-License-Identifier: AGPL-3.0-only

// このサービスについて・利用規約・プライバシーポリシーに載せる、運営者ごとの値。
// リポジトリは公開するので、個人の名前などはコードに書かず `.env` から読む（.env.example を参照）。
// サーバーでだけ読む（ブラウザの JS には入れない）。未設定なら、画面に【要記入】の印を出す

/** `.env` の値（空なら null） */
function env(value: string | undefined): string | null {
  const v = value?.trim();
  return v ? v : null;
}

export const SITE_INFO = {
  /** 運営者の名前（個人名または屋号） */
  operatorName: env(process.env.SITE_OPERATOR_NAME),
  /** 収支を公開するページの URL */
  financeReportUrl: env(process.env.SITE_FINANCE_REPORT_URL),
  /** 第一審の専属的合意管轄裁判所（例: 宇都宮地方裁判所） */
  jurisdictionCourt: env(process.env.SITE_JURISDICTION_COURT),
  /** サーバーのアクセスの記録を保存する期間（例: 90日） */
  accessLogRetention: env(process.env.SITE_ACCESS_LOG_RETENTION),
  /** サーバーの事業者と国・地域（例: 〇〇株式会社（日本）） */
  hosting: env(process.env.SITE_HOSTING),
  /** AI の事業者でのデータの扱い（保存の期間・学習に使わないことなど。確かめて書く） */
  aiDataHandling: env(process.env.SITE_AI_DATA_HANDLING),
} as const;
