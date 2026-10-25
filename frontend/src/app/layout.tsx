// SPDX-License-Identifier: AGPL-3.0-or-later

import type { Metadata, Viewport } from "next";

import { SiteFooter } from "@/components/site-footer";
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
      <body className="min-h-full flex flex-col">
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
