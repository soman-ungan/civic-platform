// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";

import { AccountSettings } from "@/components/auth/account-settings";
import { InfoPage } from "@/components/info/prose";

export const metadata: Metadata = {
  title: "アカウント",
  robots: { index: false },
};

export default function AccountPage() {
  return (
    <InfoPage title="アカウント">
      <AccountSettings />
    </InfoPage>
  );
}
