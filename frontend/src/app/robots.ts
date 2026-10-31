// SPDX-License-Identifier: AGPL-3.0-only

import type { MetadataRoute } from "next";

/** 検索エンジンへの案内。運営者の画面・アカウント・API は載せない */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: "*",
      allow: "/",
      disallow: ["/operator", "/account", "/api/"],
    },
  };
}
