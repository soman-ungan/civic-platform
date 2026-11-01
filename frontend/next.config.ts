// SPDX-License-Identifier: AGPL-3.0-only

import { existsSync } from "node:fs";
import path from "node:path";

import type { NextConfig } from "next";

import { apiInternalUrl as apiInternalUrlOf } from "./src/lib/api/base";

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

// backend の URL（サーバー側だけで使う）。ブラウザは同一オリジンの /api/* を呼び、
// 開発時は下の rewrites で backend に転送する。本番ではリバースプロキシが /api を振り分ける。
const apiInternalUrl = apiInternalUrlOf();

const nextConfig: NextConfig = {
  // 本番のイメージ（frontend/Dockerfile）は .next/standalone の server.js で動かす
  output: "standalone",
  cacheComponents: true,
  partialPrefetching: true,
  async rewrites() {
    return [
      { source: "/api/:path*", destination: `${apiInternalUrl}/api/:path*` },
    ];
  },
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
