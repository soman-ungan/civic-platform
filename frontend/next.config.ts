// SPDX-License-Identifier: AGPL-3.0-or-later

import { existsSync } from "node:fs";
import path from "node:path";

import type { NextConfig } from "next";

// .env はモノレポのルートに 1 つだけ置く方針のため、ルートの .env も読み込む。
// 既に設定済みの環境変数（デプロイ先での設定など）は上書きされない。
const rootEnv = path.resolve(process.cwd(), "..", ".env");
if (existsSync(rootEnv)) {
  process.loadEnvFile(rootEnv);
}

// AGPL 第13条: 全ページのフッターにソースコードへのリンクを出すため、本番ビルドでは必須
if (
  process.env.NODE_ENV === "production" &&
  !process.env.NEXT_PUBLIC_SOURCE_URL
) {
  throw new Error(
    "NEXT_PUBLIC_SOURCE_URL が設定されていません（.env.example を参照）",
  );
}

const nextConfig: NextConfig = {
  /* config options here */
  cacheComponents: true,
  partialPrefetching: true,
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
