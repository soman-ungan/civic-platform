// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { HeaderAccount } from "@/components/auth/header-account";
import { OfficialRecord } from "@/components/officials/record";
import { municipalityApi, officialApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import type { OfficialDetail } from "@/lib/api/types/officials";
import { pageMetadata } from "@/lib/metadata";
import { roleTitle, termLabel } from "@/lib/officials";

async function load(slug: string, id: string) {
  return Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<OfficialDetail>(officialApi(slug, id)),
  ]);
}

export async function generateMetadata({
  params,
}: PageProps<"/[municipality]/council/[id]">) {
  const { municipality: slug, id } = await params;
  const [m, official] = await load(slug, id);
  if (!m || !official) {
    return {};
  }
  return pageMetadata(
    `${official.name}（${roleTitle(m.name, official.role)}）の議会での記録`,
    `${roleTitle(m.name, official.role)}の${official.name}氏が議会で取り上げたことと、そこから生まれたトピックを、会議録の引用で見られます。`,
  );
}

/** 議員の記録（ADR 0008）: 議員 ＞ トピック（分野で絞り込む）＞ 発言の引用 */
export default function OfficialPage({
  params,
}: PageProps<"/[municipality]/council/[id]">) {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto">
      <div className="mx-auto w-full max-w-md px-4 pt-2 pb-8">
        <Suspense fallback={<OfficialSkeleton />}>
          <OfficialView params={params} />
        </Suspense>
      </div>
    </main>
  );
}

async function OfficialView({
  params,
}: Pick<PageProps<"/[municipality]/council/[id]">, "params">) {
  const { municipality: slug, id } = await params;
  const [m, official] = await load(slug, id);
  if (!m || !official) {
    notFound();
  }
  return (
    <OfficialRecord
      slug={m.slug}
      official={official}
      header={
        <>
          <div className="flex items-center justify-between gap-4">
            <Link
              href={`/${m.slug}/council`}
              className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
            >
              ← {m.name}の議会
            </Link>
            <HeaderAccount />
          </div>
          <p className="mt-1 text-xs text-zinc-500">
            {roleTitle(m.name, official.role)}
          </p>
          <h1 className="text-xl font-bold tracking-tight">{official.name}</h1>
          <p className="mt-1 text-xs text-zinc-500">
            任期 {termLabel(official)}
          </p>
        </>
      }
    />
  );
}

function OfficialSkeleton() {
  return (
    <div aria-hidden className="animate-pulse">
      <div className="mb-2 h-10 w-40 rounded bg-zinc-100 dark:bg-zinc-900" />
      <div className="mb-4 h-7 w-32 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="h-64 rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
