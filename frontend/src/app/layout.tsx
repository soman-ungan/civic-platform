// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata, Viewport } from "next";

import { SiteFooter } from "@/components/layout/site-footer";
import "./globals.css";

export const metadata: Metadata = {
  title: "市民参加プラットフォーム",
  description: "自治体の議事録・予算・決算を可視化し、市民の合意点を見つける場",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#ffffff",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="ja" className="h-full antialiased">
      {/* 画面の高さに固定し、スクロールは各ページの中で行う（ヘッダーとフッターを常に表示するため）。
          印刷（まとめの PDF）のときは高さを固定せず、全体を流す */}
      <body className="flex h-dvh flex-col overflow-hidden print:block print:h-auto print:overflow-visible">
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
