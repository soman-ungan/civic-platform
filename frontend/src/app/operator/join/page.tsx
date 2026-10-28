// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import { Suspense } from "react";

import { OperatorJoin } from "@/components/auth/operator-join";

export const metadata: Metadata = {
  title: "運営者の登録",
  robots: { index: false },
};

/** 運営者の招待のリンク（CLI の add operator が出す）から、パスキーを登録して運営者になる */
export default function OperatorJoinPage({
  searchParams,
}: PageProps<"/operator/join">) {
  return (
    <main className="mx-auto w-full max-w-md flex-1 overflow-y-auto px-4 py-8">
      <h1 className="text-lg font-semibold">運営者の登録</h1>
      <Suspense>
        <Join searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function Join({
  searchParams,
}: Pick<PageProps<"/operator/join">, "searchParams">) {
  const token = (await searchParams).token;
  return <OperatorJoin token={typeof token === "string" ? token : null} />;
}
