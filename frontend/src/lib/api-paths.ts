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
