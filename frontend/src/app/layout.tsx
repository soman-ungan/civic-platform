// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata, Viewport } from "next";
import { Suspense } from "react";

import {
  COVER_NOSCRIPT_STYLE,
  COVER_SCRIPT,
  Cover,
} from "@/components/layout/cover";
import { ChromeAutoHide } from "@/components/layout/chrome-autohide";
import { SiteFooter } from "@/components/layout/site-footer";
import { SITE_DESCRIPTION, SITE_NAME, pageMetadata } from "@/lib/metadata";
import "./globals.css";

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL;

export const metadata: Metadata = {
  ...pageMetadata(SITE_NAME, SITE_DESCRIPTION),
  // 共有したときの URL の基準（本番のドメイン）
  metadataBase: siteUrl ? new URL(siteUrl) : undefined,
  // 子のページの題は「題 | サービス名」
  title: { default: SITE_NAME, template: `%s | ${SITE_NAME}` },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#ffffff",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    // data-cover は <head> のスクリプトが描く前に付ける（React の描いたものと違ってよい）
    <html lang="ja" className="h-full antialiased" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: COVER_SCRIPT }} />
        <noscript>
          <style>{COVER_NOSCRIPT_STYLE}</style>
        </noscript>
      </head>
      {/* 画面の高さに固定し、スクロールは各ページの中で行う（ヘッダーとフッターを常に表示するため）。
          印刷（まとめの PDF）のときは高さを固定せず、全体を流す */}
      <body className="flex h-dvh flex-col overflow-hidden print:block print:h-auto print:overflow-visible">
        <Cover />
        {children}
        <SiteFooter />
        {/* ページの URL を読むので、静的な枠の外に（何も描かない） */}
        <Suspense fallback={null}>
          <ChromeAutoHide />
        </Suspense>
      </body>
    </html>
  );
}
