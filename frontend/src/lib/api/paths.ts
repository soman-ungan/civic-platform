// SPDX-License-Identifier: AGPL-3.0-only

// backend の API の URL（パス）を組み立てる。サーバー側（apiGet）とブラウザ側（apiFetch）の両方で使う

const enc = encodeURIComponent;

export const MUNICIPALITIES_API = "/api/municipalities";

/** /api/municipalities/{slug} */
export function municipalityApi(slug: string): string {
  return `${MUNICIPALITIES_API}/${enc(slug)}`;
}

/** /api/municipalities/{slug}/topics/{id} */
export function topicApi(slug: string, topicId: string): string {
  return `${municipalityApi(slug)}/topics/${enc(topicId)}`;
}

/** /api/municipalities/{slug}/proposals（トピックの提案と、自分の提案の一覧） */
export function proposalsApi(slug: string): string {
  return `${municipalityApi(slug)}/proposals`;
}

/** /api/municipalities/{slug}/recommendations（公開済みの提言） */
export function recommendationsApi(slug: string): string {
  return `${municipalityApi(slug)}/recommendations`;
}

/** パスキーでの登録とログイン（ADR 0006） */
export const AUTH_API = {
  me: "/api/auth/me",
  registerStart: "/api/auth/register/start",
  registerFinish: "/api/auth/register/finish",
  loginStart: "/api/auth/login/start",
  loginFinish: "/api/auth/login/finish",
  logout: "/api/auth/logout",
} as const;

/** 運営者の API（運営者でなければ 404） */
export const OPERATOR_MUNICIPALITIES_API = "/api/operator/municipalities";

/** /api/operator/municipalities/{slug} */
export function operatorApi(slug: string): string {
  return `${OPERATOR_MUNICIPALITIES_API}/${enc(slug)}`;
}

/** /api/operator/municipalities/{slug}/topics/{id} */
export function operatorTopicApi(slug: string, topicId: string): string {
  return `${operatorApi(slug)}/topics/${enc(topicId)}`;
}

/** /api/operator/municipalities/{slug}/statements/{id} */
export function operatorStatementApi(
  slug: string,
  statementId: string,
): string {
  return `${operatorApi(slug)}/statements/${enc(statementId)}`;
}

/** /api/operator/municipalities/{slug}/recommendations/{id}（id を省くと一覧・作成） */
export function operatorRecommendationApi(slug: string, id?: string): string {
  const base = `${operatorApi(slug)}/recommendations`;
  return id ? `${base}/${enc(id)}` : base;
}
