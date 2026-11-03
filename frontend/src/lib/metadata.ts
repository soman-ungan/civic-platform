// SPDX-License-Identifier: AGPL-3.0-only

// ページの題と説明、共有したときの表示（Open Graph・X のカード）。
// 子のページで openGraph を書くと親の openGraph はまるごと置き換わる（Next.js のメタデータは浅くまとまる）ので、
// 共通の項目はここで毎回入れる

import type { Metadata } from "next";

import { apiGet } from "@/lib/api/server";
import { municipalityApi } from "@/lib/api/paths";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";

export const SITE_NAME = "まちのこえ";
export const SITE_DESCRIPTION =
  "自治体の議会・予算・決算を分かりやすく見せ、市民の合意点を見つける場";

/** 説明の長さ（共有したときのカードに収まる程度） */
const DESCRIPTION_CHARS = 100;

function shorten(text: string): string {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > DESCRIPTION_CHARS
    ? `${flat.slice(0, DESCRIPTION_CHARS - 1)}…`
    : flat;
}

/** 題と説明を、ページの <title>・description と共有したときの表示にそろえて入れる */
export function pageMetadata(title: string, description?: string): Metadata {
  const desc = description === undefined ? undefined : shorten(description);
  return {
    title,
    description: desc,
    openGraph: {
      type: "website",
      locale: "ja_JP",
      siteName: SITE_NAME,
      title,
      description: desc,
    },
    twitter: { card: "summary", title, description: desc },
  };
}

/** 自治体のページの題（「大田原市のお金」など）。自治体が無ければ空（ページの側で 404 にする） */
export async function municipalityMetadata(
  params: Promise<{ municipality: string }>,
  title: (name: string) => string,
  description: (name: string) => string,
): Promise<Metadata> {
  const { municipality: slug } = await params;
  const m = await apiGet<MunicipalityDetail>(municipalityApi(slug));
  return m ? pageMetadata(title(m.name), description(m.name)) : {};
}
