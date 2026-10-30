// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { OperatorLink } from "@/components/auth/operator-link";
import {
  RecommendationItems,
  RecommendationTopics,
} from "@/components/recommendations/items";
import { PrintButton } from "@/components/recommendations/print-button";
import { municipalityApi, recommendationsApi } from "@/lib/api/paths";
import { apiGet } from "@/lib/api/server";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import type {
  RecommendationDetail,
  RecommendationLink,
} from "@/lib/api/types/recommendations";
import { RESIDENCY_ORDER, RESIDENCY_SHORT_LABELS } from "@/lib/labels";
import { formatDate, ratioLabel } from "@/lib/recommendations";

export default function RecommendationPage({
  params,
}: PageProps<"/[municipality]/recommendations/[id]">) {
  // 見出しは固定のヘッダーにし、中身だけをスクロールする。印刷のときは全体を1枚の文書として流す
  return (
    <main className="flex min-h-0 flex-1 flex-col print:block">
      <Suspense fallback={<Skeleton />}>
        <RecommendationView params={params} />
      </Suspense>
    </main>
  );
}

async function RecommendationView({
  params,
}: Pick<PageProps<"/[municipality]/recommendations/[id]">, "params">) {
  const { municipality: slug, id } = await params;
  const [municipality, r] = await Promise.all([
    apiGet<MunicipalityDetail>(municipalityApi(slug)),
    apiGet<RecommendationDetail>(
      `${recommendationsApi(slug)}/${encodeURIComponent(id)}`,
    ),
  ]);
  if (!municipality || !r) {
    notFound();
  }
  const topicTitles = new Map(r.topics.map((t) => [t.id, t.title]));
  const consensus = r.items.filter((i) => i.kind === "consensus");
  const divided = r.items.filter((i) => i.kind === "divided");
  const labels = RESIDENCY_ORDER.map(
    (key) => [key, RESIDENCY_SHORT_LABELS[key]] as [string, string],
  );

  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800 print:border-0">
        <div className="mx-auto w-full max-w-md px-4 pb-3 print:max-w-none">
          <div className="flex items-center justify-between gap-4 print:hidden">
            <Link
              href={`/${municipality.slug}/recommendations`}
              className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
            >
              ← {municipality.name}の提言
            </Link>
            <OperatorLink />
          </div>
          <p className="mb-0.5 text-xs text-zinc-500">
            {municipality.name}の市民の投票からの提言
            {r.published_at && ` ・ ${formatDate(r.published_at)}`}
          </p>
          <h1 className="text-lg leading-snug font-bold">{r.title}</h1>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain print:overflow-visible">
        <div className="mx-auto flex w-full max-w-md flex-col gap-8 px-4 pt-4 pb-8 print:max-w-none">
          <div className="flex flex-col gap-3">
            {r.preface && (
              <p className="text-sm leading-7 text-zinc-700 dark:text-zinc-300">
                {r.preface}
              </p>
            )}
            <div>
              <PrintButton />
            </div>
          </div>

          <Section title={`合意点（${consensus.length}件）`}>
            <p className="mb-3 text-xs leading-5 text-zinc-500">
              どの意見グループでも、{ratioLabel(r.consensus_threshold)}
              以上が賛成した意見です。
            </p>
            {consensus.length > 0 ? (
              <RecommendationItems items={consensus} topics={topicTitles} />
            ) : (
              <Nothing />
            )}
          </Section>

          <Section title={`意見が分かれた点（${divided.length}件）`}>
            <p className="mb-3 text-xs leading-5 text-zinc-500">
              あるグループでは{ratioLabel(r.divided_threshold)}
              以上が賛成し、別のグループでは{ratioLabel(r.divided_threshold)}
              以上が反対した意見です。合意点だけでは伝わらない、議論の全体を残すために載せています。
            </p>
            {divided.length > 0 ? (
              <RecommendationItems items={divided} topics={topicTitles} />
            ) : (
              <Nothing />
            )}
          </Section>

          {r.topics.length > 0 && (
            <Section title="元のトピック">
              <RecommendationTopics
                slug={municipality.slug}
                topics={r.topics}
                labels={labels}
              />
              <p className="mt-2 text-xs leading-5 text-zinc-500">
                関わり方の人数は、少ない区分を「—」で隠しています（ほかの区分からの逆算も防ぎます）。
              </p>
            </Section>
          )}

          <Links
            slug={municipality.slug}
            title="まとめた提言"
            kind="recommendations"
            links={r.children}
          />
          <Links
            slug={municipality.slug}
            title="この提言をまとめた提言"
            kind="recommendations"
            links={r.parents}
          />
          <Links
            slug={municipality.slug}
            title="この提言から始まった第2回のトピック"
            kind="topics"
            links={r.second_round}
          />

          <p className="text-xs leading-5 text-zinc-500">
            グループ
            A・B…は、元のトピックごとの投票の傾向で分けた意見のグループです（年齢や住む地区などで分けたものではありません）。トピックが違えば、同じ記号でも別のグループです。
          </p>
          <p className="text-xs leading-5 text-zinc-500">
            この提言の文は、市民の投票した意見の原文です。見出しと前書きのほかは、AI
            も運営者も書き足していません。数字は提言を作ったときのものです。
          </p>
        </div>
      </div>
    </>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <h2 className="mb-2 text-base font-bold">{title}</h2>
      {children}
    </section>
  );
}

function Nothing() {
  return <p className="text-sm text-zinc-500">ありません。</p>;
}

function Links({
  slug,
  title,
  kind,
  links,
}: {
  slug: string;
  title: string;
  kind: "recommendations" | "topics";
  links: RecommendationLink[];
}) {
  if (links.length === 0) return null;
  return (
    <Section title={title}>
      <ul className="flex flex-col gap-1 text-sm">
        {links.map((l) => (
          <li key={l.id}>
            <Link
              href={`/${slug}/${kind}/${l.id}`}
              className="underline underline-offset-2"
            >
              {l.title}
            </Link>
          </li>
        ))}
      </ul>
    </Section>
  );
}

function Skeleton() {
  return (
    <div
      aria-hidden
      className="mx-auto w-full max-w-md animate-pulse px-4 py-3"
    >
      <div className="mb-4 h-4 w-40 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-6 w-3/4 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="h-48 rounded-3xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
